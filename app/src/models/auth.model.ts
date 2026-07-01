export type UserRole = 'player' | 'admin';

export interface AuthClaims {
    sub: string;
    email: string;
    role: UserRole;
    exp?: number;
}

export interface CurrentUser {
    id: string;
    email: string;
    role: UserRole;
}
