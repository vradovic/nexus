import { computed, inject, Service, signal } from '@angular/core';
import { LoginModel, LoginResponse } from '../models/login.model';
import { HttpClient } from '@angular/common/http';
import { environment } from '../environments/environment';
import { RegisterModel } from '../models/register.model';
import { tap } from 'rxjs';

const TOKEN_KEY = 'access_token';

@Service()
export class AuthService {
    private readonly http = inject(HttpClient);
    private readonly token = signal<string | null>(sessionStorage.getItem(TOKEN_KEY));
    readonly accessToken = this.token.asReadonly();

    isAuthenticated = computed(() => !!this.token());

    login(data: LoginModel) {
        return this.http.post<LoginResponse>(`${environment.authApiUrl}/login`, data).pipe(
            tap(response => this.setToken(response.access_token))
        );
    }

    logout() {
        this.token.set(null);
        sessionStorage.removeItem(TOKEN_KEY);
    }

    register(data: RegisterModel) {
        return this.http.post<void>(`${environment.authApiUrl}/register`, data);
    }

    getToken() {
        return this.token();
    }

    private setToken(token: string) {
        this.token.set(token);
        sessionStorage.setItem(TOKEN_KEY, token);
    }
}
