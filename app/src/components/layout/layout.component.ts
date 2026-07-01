import { Component, inject } from '@angular/core';
import { Router, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { AuthService } from '../../services/auth.service';
import { MatchmakingService } from '../../services/matchmaking.service';
import { RealtimeService } from '../../services/realtime.service';

@Component({
    selector: 'app-layout',
    imports: [RouterOutlet, RouterLink, RouterLinkActive],
    templateUrl: './layout.component.html',
    styleUrl: './layout.component.scss',
})
export class LayoutComponent {
    readonly auth = inject(AuthService);
    readonly realtime = inject(RealtimeService);
    private readonly matchmaking = inject(MatchmakingService);
    private readonly router = inject(Router);

    logout() {
        this.matchmaking.reset();
        this.realtime.disconnect();
        this.auth.logout();
        void this.router.navigate(['/login']);
    }
}
