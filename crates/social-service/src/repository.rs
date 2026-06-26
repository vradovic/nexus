use nexus_shared::AppError;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    BlockedUser, ChatMessage, Friend, FriendRequest, FriendRequestView, UserProfile,
    UserRegisteredEvent,
};

#[derive(Clone)]
pub struct UserProfileRepository {
    db: PgPool,
}

#[derive(Clone)]
pub struct ChatRepository {
    db: PgPool,
}

impl UserProfileRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn upsert_user_profile(&self, event: &UserRegisteredEvent) -> Result<(), AppError> {
        sqlx::query(
            r#"
            insert into user_profiles (id, first_name, last_name)
            values ($1, $2, $3)
            on conflict (id) do update
            set first_name = excluded.first_name,
                last_name = excluded.last_name
            "#,
        )
        .bind(event.user_id)
        .bind(&event.first_name)
        .bind(&event.last_name)
        .execute(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        Ok(())
    }

    pub async fn find_user_profile_by_id(&self, id: Uuid) -> Result<Option<UserProfile>, AppError> {
        sqlx::query_as::<_, UserProfile>(
            r#"
            select id, first_name, last_name
            from user_profiles
            where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }

    pub async fn create_friend_request(
        &self,
        requester_id: Uuid,
        recipient_id: Uuid,
    ) -> Result<FriendRequest, AppError> {
        sqlx::query_as::<_, FriendRequest>(
            r#"
            insert into friend_requests (id, requester_id, recipient_id, status)
            values ($1, $2, $3, 'pending')
            returning id, requester_id, recipient_id, status
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(requester_id)
        .bind(recipient_id)
        .fetch_one(&self.db)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(db_error)
                if db_error.is_unique_violation()
                    || db_error.constraint() == Some("friend_requests_check") =>
            {
                AppError::conflict("friend request already exists")
            }
            sqlx::Error::Database(db_error) if db_error.is_foreign_key_violation() => {
                AppError::not_found("user profile not found")
            }
            _ => AppError::internal("database operation failed"),
        })
    }

    pub async fn friendship_exists(
        &self,
        user_id: Uuid,
        friend_id: Uuid,
    ) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            select exists (
                select 1
                from friendships
                where least(user_a_id, user_b_id) = least($1::uuid, $2::uuid)
                  and greatest(user_a_id, user_b_id) = greatest($1::uuid, $2::uuid)
            )
            "#,
        )
        .bind(user_id)
        .bind(friend_id)
        .fetch_one(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        Ok(exists)
    }

    pub async fn block_exists_between(
        &self,
        user_id: Uuid,
        other_user_id: Uuid,
    ) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            select exists (
                select 1
                from user_blocks
                where (blocker_id = $1 and blocked_id = $2)
                   or (blocker_id = $2 and blocked_id = $1)
            )
            "#,
        )
        .bind(user_id)
        .bind(other_user_id)
        .fetch_one(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        Ok(exists)
    }

    pub async fn list_friends(&self, user_id: Uuid) -> Result<Vec<Friend>, AppError> {
        sqlx::query_as::<_, Friend>(
            r#"
            select
                friendships.id as friendship_id,
                case
                    when friendships.user_a_id = $1 then user_b.id
                    else user_a.id
                end as friend_id,
                case
                    when friendships.user_a_id = $1 then user_b.first_name
                    else user_a.first_name
                end as first_name,
                case
                    when friendships.user_a_id = $1 then user_b.last_name
                    else user_a.last_name
                end as last_name
            from friendships
            join user_profiles user_a on user_a.id = friendships.user_a_id
            join user_profiles user_b on user_b.id = friendships.user_b_id
            where friendships.user_a_id = $1
               or friendships.user_b_id = $1
            order by first_name, last_name, friend_id
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }

    pub async fn block_user(
        &self,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<BlockedUser, AppError> {
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|_| AppError::internal("database operation failed"))?;

        sqlx::query(
            r#"
            update friend_requests
            set status = 'declined',
                responded_at = now()
            where status = 'pending'
              and (
                  (requester_id = $1 and recipient_id = $2)
                  or (requester_id = $2 and recipient_id = $1)
              )
            "#,
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        sqlx::query(
            r#"
            delete from friendships
            where least(user_a_id, user_b_id) = least($1::uuid, $2::uuid)
              and greatest(user_a_id, user_b_id) = greatest($1::uuid, $2::uuid)
            "#,
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        let blocked_user = sqlx::query_as::<_, BlockedUser>(
            r#"
            with inserted as (
                insert into user_blocks (id, blocker_id, blocked_id)
                values ($1, $2, $3)
                on conflict (blocker_id, blocked_id) do update
                set blocked_id = excluded.blocked_id
                returning id, blocked_id
            )
            select
                inserted.id as block_id,
                inserted.blocked_id as blocked_user_id,
                user_profiles.first_name,
                user_profiles.last_name
            from inserted
            join user_profiles on user_profiles.id = inserted.blocked_id
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(blocker_id)
        .bind(blocked_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(db_error)
                if db_error.constraint() == Some("user_blocks_check") =>
            {
                AppError::bad_request("cannot block yourself")
            }
            sqlx::Error::Database(db_error) if db_error.is_foreign_key_violation() => {
                AppError::not_found("user profile not found")
            }
            _ => AppError::internal("database operation failed"),
        })?;

        tx.commit()
            .await
            .map_err(|_| AppError::internal("database operation failed"))?;

        Ok(blocked_user)
    }

    pub async fn unblock_user(&self, blocker_id: Uuid, blocked_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            delete from user_blocks
            where blocker_id = $1
              and blocked_id = $2
            "#,
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        Ok(())
    }

    pub async fn list_blocked_users(&self, blocker_id: Uuid) -> Result<Vec<BlockedUser>, AppError> {
        sqlx::query_as::<_, BlockedUser>(
            r#"
            select
                user_blocks.id as block_id,
                user_blocks.blocked_id as blocked_user_id,
                user_profiles.first_name,
                user_profiles.last_name
            from user_blocks
            join user_profiles on user_profiles.id = user_blocks.blocked_id
            where user_blocks.blocker_id = $1
            order by user_profiles.first_name, user_profiles.last_name, user_blocks.blocked_id
            "#,
        )
        .bind(blocker_id)
        .fetch_all(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }

    pub async fn list_incoming_friend_requests(
        &self,
        recipient_id: Uuid,
    ) -> Result<Vec<FriendRequestView>, AppError> {
        self.list_friend_requests(
            r#"
            select
                friend_requests.id,
                friend_requests.requester_id,
                requester.first_name as requester_first_name,
                requester.last_name as requester_last_name,
                friend_requests.recipient_id,
                recipient.first_name as recipient_first_name,
                recipient.last_name as recipient_last_name,
                friend_requests.status
            from friend_requests
            join user_profiles requester on requester.id = friend_requests.requester_id
            join user_profiles recipient on recipient.id = friend_requests.recipient_id
            where friend_requests.recipient_id = $1
              and friend_requests.status = 'pending'
            order by friend_requests.created_at desc
            "#,
            recipient_id,
        )
        .await
    }

    pub async fn list_outgoing_friend_requests(
        &self,
        requester_id: Uuid,
    ) -> Result<Vec<FriendRequestView>, AppError> {
        self.list_friend_requests(
            r#"
            select
                friend_requests.id,
                friend_requests.requester_id,
                requester.first_name as requester_first_name,
                requester.last_name as requester_last_name,
                friend_requests.recipient_id,
                recipient.first_name as recipient_first_name,
                recipient.last_name as recipient_last_name,
                friend_requests.status
            from friend_requests
            join user_profiles requester on requester.id = friend_requests.requester_id
            join user_profiles recipient on recipient.id = friend_requests.recipient_id
            where friend_requests.requester_id = $1
              and friend_requests.status = 'pending'
            order by friend_requests.created_at desc
            "#,
            requester_id,
        )
        .await
    }

    async fn list_friend_requests(
        &self,
        query: &str,
        user_id: Uuid,
    ) -> Result<Vec<FriendRequestView>, AppError> {
        sqlx::query_as::<_, FriendRequestView>(query)
            .bind(user_id)
            .fetch_all(&self.db)
            .await
            .map_err(|_| AppError::internal("database operation failed"))
    }

    pub async fn accept_friend_request(
        &self,
        request_id: Uuid,
        recipient_id: Uuid,
    ) -> Result<Option<FriendRequest>, AppError> {
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|_| AppError::internal("database operation failed"))?;

        let request = sqlx::query_as::<_, FriendRequest>(
            r#"
            update friend_requests
            set status = 'accepted',
                responded_at = now()
            where id = $1
              and recipient_id = $2
              and status = 'pending'
            returning id, requester_id, recipient_id, status
            "#,
        )
        .bind(request_id)
        .bind(recipient_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        let Some(request) = request else {
            tx.rollback()
                .await
                .map_err(|_| AppError::internal("database operation failed"))?;
            return Ok(None);
        };

        let blocked = sqlx::query_scalar::<_, bool>(
            r#"
            select exists (
                select 1
                from user_blocks
                where (blocker_id = $1 and blocked_id = $2)
                   or (blocker_id = $2 and blocked_id = $1)
            )
            "#,
        )
        .bind(request.requester_id)
        .bind(request.recipient_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        if blocked {
            tx.rollback()
                .await
                .map_err(|_| AppError::internal("database operation failed"))?;
            return Err(AppError::forbidden(
                "friend requests are disabled between blocked users",
            ));
        }

        sqlx::query(
            r#"
            insert into friendships (id, user_a_id, user_b_id)
            values ($1, $2, $3)
            on conflict do nothing
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(request.requester_id)
        .bind(request.recipient_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::internal("database operation failed"))?;

        tx.commit()
            .await
            .map_err(|_| AppError::internal("database operation failed"))?;

        Ok(Some(request))
    }

    pub async fn decline_friend_request(
        &self,
        request_id: Uuid,
        recipient_id: Uuid,
    ) -> Result<Option<FriendRequest>, AppError> {
        sqlx::query_as::<_, FriendRequest>(
            r#"
            update friend_requests
            set status = 'declined',
                responded_at = now()
            where id = $1
              and recipient_id = $2
              and status = 'pending'
            returning id, requester_id, recipient_id, status
            "#,
        )
        .bind(request_id)
        .bind(recipient_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }
}

impl ChatRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_message(
        &self,
        channel: &str,
        sender_id: Uuid,
        body: &str,
    ) -> Result<ChatMessage, AppError> {
        sqlx::query_as::<_, ChatMessage>(
            r#"
            insert into chat_messages (id, channel, sender_id, body)
            values ($1, $2, $3, $4)
            returning id, channel, sender_id, body, created_at::text as created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(channel)
        .bind(sender_id)
        .bind(body)
        .fetch_one(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }

    pub async fn list_recent_messages(
        &self,
        channel: &str,
        limit: i64,
    ) -> Result<Vec<ChatMessage>, AppError> {
        sqlx::query_as::<_, ChatMessage>(
            r#"
            select id, channel, sender_id, body, created_at
            from (
                select id, channel, sender_id, body, created_at::text as created_at, created_at as sort_created_at
                from chat_messages
                where channel = $1
                order by created_at desc
                limit $2
            ) recent_messages
            order by sort_created_at asc
            "#,
        )
        .bind(channel)
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .map_err(|_| AppError::internal("database operation failed"))
    }
}
