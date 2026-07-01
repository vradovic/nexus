import { Component, inject, OnInit, signal } from '@angular/core';
import { AuthService } from '../../services/auth.service';
import { RouterLink } from "@angular/router";
import { SocialService } from '../../services/social.service';
import { Profile } from '../../models/profile.model';

@Component({
  selector: 'app-home',
  imports: [RouterLink],
  templateUrl: './home.component.html',
  styleUrl: './home.component.scss',
})
export class HomeComponent implements OnInit {
  private readonly authService = inject(AuthService);
  private readonly socialService = inject(SocialService);

  isAuthenticated = this.authService.isAuthenticated;
  profile = signal<Profile | null>(null);

  ngOnInit(): void {
    if (this.isAuthenticated()) {
      this.socialService.me().subscribe(profile => this.profile.set(profile));
    }
  }

  onLogOut() {
    this.authService.logout();
  }
}
