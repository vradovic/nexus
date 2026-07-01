import { Component, inject } from '@angular/core';
import { AuthService } from '../../services/auth.service';
import { RouterLink } from "@angular/router";
import { SocialService } from '../../services/social.service';

@Component({
  selector: 'app-home',
  imports: [RouterLink],
  templateUrl: './home.component.html',
  styleUrl: './home.component.scss',
})
export class HomeComponent {
  private readonly authService = inject(AuthService);
  private readonly socialService = inject(SocialService);

  isAuthenticated = this.authService.isAuthenticated;
  profile = this.socialService.profile;

  onLogOut() {
    this.authService.logout();
  }
}
