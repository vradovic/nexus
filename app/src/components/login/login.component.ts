import { Component, inject, signal } from '@angular/core';
import { form, FormField } from '@angular/forms/signals';
import { LoginModel } from '../../models/login.model';
import { AuthService } from '../../services/auth.service';
import { Router, RouterLink } from "@angular/router";
import { routes } from '../../app/app.routes';

@Component({
  selector: 'app-login',
  imports: [FormField, RouterLink],
  templateUrl: './login.component.html',
  styleUrl: './login.component.scss',
})
export class LoginComponent {
  loginModel = signal<LoginModel>({
    email: '',
    password: '',
  });

  loginForm = form(this.loginModel);

  authService = inject(AuthService);
  router = inject(Router);

  onSubmit(event: Event) {
    event.preventDefault();

    const data = this.loginModel();
    this.authService.login(data).subscribe({
      next: () => {
        this.router.navigate(['/']);
      },
      error: () => {
        alert('Invalid username or password.');
      },
    });
  }
}
