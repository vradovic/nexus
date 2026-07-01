import { Component, inject, signal } from '@angular/core';
import { form, FormField } from '@angular/forms/signals';
import { Router, RouterLink } from '@angular/router';
import { RegisterModel } from '../../models/register.model';
import { AuthService } from '../../services/auth.service';
import { HttpErrorResponse } from '@angular/common/http';

@Component({
  selector: 'app-register.component',
  imports: [FormField, RouterLink],
  templateUrl: './register.component.html',
  styleUrl: './register.component.scss',
})
export class RegisterComponent {
  private readonly authService = inject(AuthService);
  private readonly router = inject(Router);

  registerModel = signal<RegisterModel>({
    email: '',
    username: '',
    firstName: '',
    lastName: '',
    password: '',
  });

  registerForm = form(this.registerModel);

  onSubmit(event: Event) {
    event.preventDefault();

    const data = this.registerModel();
    this.authService.register(data).subscribe({
      next: () => {
        alert(`Registration successful!`);
        this.router.navigate(['/']);
      },
      error: (err: HttpErrorResponse) => {
        if (err.status === 409) {
          alert('Username or email already exists.');
        } else {
          alert('Failed to register.');
        }
      },
    });
  }
}
