import { Component, signal } from '@angular/core';
import { form } from '@angular/forms/signals';
import { RegisterModel } from '../../models/register.model';

@Component({
  selector: 'app-register.component',
  imports: [],
  templateUrl: './register.component.html',
  styleUrl: './register.component.scss',
})
export class RegisterComponent {
  loginModel = signal<RegisterModel>({
    email: '',
    username: '',
    firstName: '',
    lastName: '',
    password: '',
  });

  loginForm = form(this.loginModel);
}
