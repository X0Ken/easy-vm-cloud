import { Routes } from '@angular/router';

export const ME_ROUTES: Routes = [
  {
    path: 'profile',
    loadComponent: () => import('./profile/profile.component').then(m => m.ProfileComponent),
  }
];
