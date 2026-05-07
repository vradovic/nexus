use nexus_shared::AppError;
use redis::{AsyncCommands, Client};
use serde::{Serialize, de::DeserializeOwned};

pub async fn write_json<T>(redis_client: &Client, key: &str, value: &T) -> Result<(), AppError>
where
    T: Serialize,
{
    let payload = serde_json::to_string(value)
        .map_err(|_| AppError::internal("failed to serialize value for redis"))?;
    let mut connection = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::internal("failed to connect to redis"))?;

    connection
        .set::<&str, String, ()>(key, payload)
        .await
        .map_err(|_| AppError::internal("failed to write value to redis"))?;

    Ok(())
}

pub async fn read_json<T>(redis_client: &Client, key: &str) -> Result<Option<T>, AppError>
where
    T: DeserializeOwned,
{
    let mut connection = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::internal("failed to connect to redis"))?;

    let payload = connection
        .get::<&str, Option<String>>(key)
        .await
        .map_err(|_| AppError::internal("failed to read value from redis"))?;

    payload
        .map(|value| {
            serde_json::from_str::<T>(&value)
                .map_err(|_| AppError::internal("failed to deserialize value from redis"))
        })
        .transpose()
}
