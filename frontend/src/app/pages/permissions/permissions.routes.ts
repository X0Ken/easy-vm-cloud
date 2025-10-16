import { Routes } from '@angular/router';

export const PERMISSIONS_ROUTES: Routes = [
  {
    path: 'permissions',
    loadComponent: () => import('./permissions/permissions.component').then(m => m.PermissionsComponent),
  },
  {
    path: 'roles',
    loadComponent: () => import('./roles/roles.component').then(m => m.RolesComponent),
  },
  {
    path: 'users',
    loadComponent: () => import('./users/users.component').then(m => m.UsersComponent),
  },
  {
    path: 'departments',
    loadComponent: () => import('./departments/departments.component').then(m => m.DepartmentsComponent),
  },
  {
    path: 'user-departments',
    loadComponent: () => import('./user-departments/user-departments.component').then(m => m.UserDepartmentsComponent),
  }
];
