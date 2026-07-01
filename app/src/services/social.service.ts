import { HttpClient } from '@angular/common/http';
import { effect, inject, Service, signal } from '@angular/core';
import { environment } from '../environments/environment';
import { AuthService } from './auth.service';
import { Profile } from '../models/profile.model';

@Service()
export class SocialService {
    private readonly http = inject(HttpClient);
    private readonly authService = inject(AuthService);

    readonly profile = signal<Profile | null>(null);

    constructor() {
        effect(() => {
            const token = this.authService.accessToken();

            if (!token) {
                this.profile.set(null);
                return;
            }

            this.me().subscribe({
                next: profile => this.profile.set(profile),
                error: () => console.error('Failed to fetch user profile.'),
            });
        });
    }

    me() {
        return this.http.get<Profile>(`${environment.socialApiUrl}/me`);
    }
}
