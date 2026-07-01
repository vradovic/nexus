export interface RegisterModel {
    email: string;
    username: string;
    firstName: string;
    lastName: string;
    password: string;
}

export interface RegisterResponse {
    id: string;
    email: string;
    username: string;
}
