import { Routes } from '@angular/router';

export const STORAGE_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () => import('./storage.component').then(m => m.StorageComponent)
  }
];
