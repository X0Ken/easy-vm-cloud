import { Routes } from '@angular/router';

export const VMS_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () => import('./vms.component').then(m => m.VmsComponent)
  }
];
