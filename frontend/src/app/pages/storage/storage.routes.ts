import { Routes } from '@angular/router';

export const STORAGE_ROUTES: Routes = [
  {
    path: '',
    redirectTo: 'pools',
    pathMatch: 'full',
  },
  {
    path: 'pools',
    loadComponent: () =>
      import('./storage-pools/storage-pools.component').then((m) => m.StoragePoolsComponent),
  },
  {
    path: 'volumes',
    loadComponent: () =>
      import('./storage-volumes/storage-volumes.component').then((m) => m.StorageVolumesComponent),
  },
  {
    path: 'snapshots',
    loadComponent: () =>
      import('./snapshots/snapshots.component').then((m) => m.SnapshotsComponent),
  },
];
