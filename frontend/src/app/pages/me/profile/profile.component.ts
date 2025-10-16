import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzDescriptionsModule } from 'ng-zorro-antd/descriptions';
import { NzTagModule } from 'ng-zorro-antd/tag';

@Component({
  selector: 'app-profile',
  standalone: true,
  imports: [
    CommonModule,
    NzCardModule,
    NzDescriptionsModule,
    NzTagModule
  ],
  templateUrl: './profile.component.html',
  styleUrls: ['./profile.component.scss']
})
export class ProfileComponent {
  userInfo = {
    id: 1,
    username: 'admin',
    email: 'admin@example.com',
    is_active: true,
    roles: ['超级管理员'],
    permissions: ['user:read', 'user:create', 'user:update', 'user:delete', 'role:read', 'role:create']
  };
}
