import { Routes } from '@angular/router';
import { AuthGuard } from './guards/auth.guard';

export const routes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: '/login' },
  { path: 'login', loadChildren: () => import('./pages/login/login.routes').then(m => m.LOGIN_ROUTES) },
  {
    path: '',
    canActivate: [AuthGuard],
    children: [
      {
        path: 'welcome',
        loadChildren: () => import('./pages/welcome/welcome.routes').then(m => m.WELCOME_ROUTES)
      },
      {
        path: 'permissions',
        loadChildren: () => import('./pages/permissions/permissions.routes').then(m => m.PERMISSIONS_ROUTES)
      },
      {
        path: 'me',
        loadChildren: () => import('./pages/me/me.routes').then(m => m.ME_ROUTES)
      },
      {
        path: 'system',
        loadChildren: () => import('./pages/system/system.routes').then(m => m.SYSTEM_ROUTES)
      },
      {
        path: 'vms',
        loadChildren: () => import('./pages/vms/vms.routes').then(m => m.VMS_ROUTES)
      },
      {
        path: 'storage',
        loadChildren: () => import('./pages/storage/storage.routes').then(m => m.STORAGE_ROUTES)
      },
      {
        path: 'network',
        loadChildren: () => import('./pages/network/network.routes').then(m => m.NETWORK_ROUTES)
      },
      {
        path: 'nodes',
        loadChildren: () => import('./pages/nodes/nodes.routes').then(m => m.NODES_ROUTES)
      }
    ]
  }
];
