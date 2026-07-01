import { Routes } from '@angular/router';
import { AdminComponent } from '../components/admin/admin.component';
import { FriendsComponent } from '../components/friends/friends.component';
import { GameComponent } from '../components/game/game.component';
import { LayoutComponent } from '../components/layout/layout.component';
import { LobbyComponent } from '../components/lobby/lobby.component';
import { LoginComponent } from '../components/login/login.component';
import { RegisterComponent } from '../components/register/register.component';
import { adminGuard, authGuard } from './auth.guard';

export const routes: Routes = [
    { path: 'login', component: LoginComponent },
    { path: 'register', component: RegisterComponent },
    {
        path: '',
        component: LayoutComponent,
        canActivate: [authGuard],
        children: [
            { path: '', component: LobbyComponent },
            { path: 'game', component: GameComponent },
            { path: 'friends', component: FriendsComponent },
            { path: 'admin', component: AdminComponent, canActivate: [adminGuard] },
        ],
    },
    { path: '**', redirectTo: '' },
];
