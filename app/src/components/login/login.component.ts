import { Component, inject, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { AuthService } from '../../services/auth.service';
import { LoginModel } from '../../models/login.model';

@Component({
    selector: 'app-login',
    imports: [RouterLink],
    templateUrl: './login.component.html',
    styleUrl: './login.component.scss',
})
export class LoginComponent {
    private readonly authService = inject(AuthService);
    private readonly router = inject(Router);

    readonly model = signal<LoginModel>({ email: '', password: '' });
    readonly busy = signal(false);
    readonly error = signal('');

    update(field: keyof LoginModel, event: Event) {
        const value = (event.target as HTMLInputElement).value;
        this.model.update(model => ({ ...model, [field]: value }));
    }

    onSubmit(event: Event) {
        event.preventDefault();

        if (this.busy()) {
            return;
        }

        this.busy.set(true);
        this.error.set('');

        this.authService.login(this.model()).subscribe({
            next: () => void this.router.navigate(['/']),
            error: () => {
                this.error.set('Invalid email or password.');
                this.busy.set(false);
            },
        });
    }
}
