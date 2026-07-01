import { HttpErrorResponse } from '@angular/common/http';
import { Component, inject, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { switchMap } from 'rxjs';
import { RegisterModel } from '../../models/register.model';
import { AuthService } from '../../services/auth.service';

@Component({
    selector: 'app-register',
    imports: [RouterLink],
    templateUrl: './register.component.html',
    styleUrl: './register.component.scss',
})
export class RegisterComponent {
    private readonly authService = inject(AuthService);
    private readonly router = inject(Router);

    readonly model = signal<RegisterModel>({
        email: '',
        username: '',
        firstName: '',
        lastName: '',
        password: '',
    });
    readonly busy = signal(false);
    readonly error = signal('');

    update(field: keyof RegisterModel, event: Event) {
        const value = (event.target as HTMLInputElement).value;
        this.model.update(model => ({ ...model, [field]: value }));
    }

    onSubmit(event: Event) {
        event.preventDefault();

        if (this.busy()) {
            return;
        }

        const data = this.model();
        this.busy.set(true);
        this.error.set('');

        this.authService.register(data).pipe(
            switchMap(() => this.authService.login({ email: data.email, password: data.password })),
        ).subscribe({
            next: () => void this.router.navigate(['/']),
            error: (err: HttpErrorResponse) => {
                this.error.set(err.status === 409 ? 'Username or email already exists.' : 'Failed to register.');
                this.busy.set(false);
            },
        });
    }
}
