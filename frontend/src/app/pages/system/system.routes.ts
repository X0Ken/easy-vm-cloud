import { Routes } from '@angular/router';

export const SYSTEM_ROUTES: Routes = [
  {
    path: 'settings',
    loadComponent: () => import('./settings/settings.component').then(m => m.SettingsComponent),
  }
];
