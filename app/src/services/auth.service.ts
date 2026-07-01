import { HttpClient } from '@angular/common/http';
import { computed, inject, Injectable, signal } from '@angular/core';
import { jwtDecode } from 'jwt-decode';
import { tap } from 'rxjs';
import { environment } from '../environments/environment';
import { AuthClaims } from '../models/auth.model';
import { LoginModel, LoginResponse } from '../models/login.model';
import { RegisterModel } from '../models/register.model';

const TOKEN_KEY = 'nexus_access_token';

@Injectable({ providedIn: 'root' })
export class AuthService {
    private readonly http = inject(HttpClient);
    private readonly token = signal<string | null>(sessionStorage.getItem(TOKEN_KEY));

    readonly accessToken = this.token.asReadonly();
    readonly claims = computed(() => decodeClaims(this.token()));
    readonly currentUser = computed(() => {
        const claims = this.claims();

        return claims
            ? {
                  id: claims.sub,
                  email: claims.email,
                  role: claims.role,
              }
            : null;
    });
    readonly isAuthenticated = computed(() => !!this.currentUser());
    readonly isAdmin = computed(() => this.currentUser()?.role === 'admin');
    readonly displayName = computed(() => this.currentUser()?.email ?? 'not signed in');
    readonly userColor = computed(() => colorFromId(this.currentUser()?.id ?? ''));

    login(data: LoginModel) {
        return this.http.post<LoginResponse>(`${environment.authApiUrl}/login`, data).pipe(
            tap(response => this.setToken(response.access_token)),
        );
    }

    logout() {
        this.token.set(null);
        sessionStorage.removeItem(TOKEN_KEY);
    }

    register(data: RegisterModel) {
        return this.http.post<void>(`${environment.authApiUrl}/register`, {
            email: data.email,
            username: data.username,
            first_name: data.firstName,
            last_name: data.lastName,
            password: data.password,
        });
    }

    getToken() {
        return this.token();
    }

    private setToken(token: string) {
        this.token.set(token);
        sessionStorage.setItem(TOKEN_KEY, token);
    }
}

function decodeClaims(token: string | null): AuthClaims | null {
    if (!token) {
        return null;
    }

    try {
        const claims = jwtDecode<AuthClaims>(token);

        if (!claims.sub || !claims.email || !claims.role) {
            return null;
        }

        if (claims.exp && claims.exp * 1000 <= Date.now()) {
            return null;
        }

        return claims;
    } catch {
        return null;
    }
}

function colorFromId(id: string) {
    if (!id) {
        return '#9aa4b1';
    }

    let hash = 0;
    for (let index = 0; index < id.length; index += 1) {
        hash = (hash * 31 + id.charCodeAt(index)) % 360;
    }

    return `hsl(${hash} 76% 42%)`;
}
