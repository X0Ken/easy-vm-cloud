import { Routes } from '@angular/router';

export const NETWORK_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () => import('./network.component').then(m => m.NetworkComponent)
  }
];
