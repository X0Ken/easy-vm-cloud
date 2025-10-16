import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzFormModule } from 'ng-zorro-antd/form';
import { NzInputModule } from 'ng-zorro-antd/input';
import { NzSwitchModule } from 'ng-zorro-antd/switch';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzSelectModule } from 'ng-zorro-antd/select';
import { FormsModule } from '@angular/forms';

@Component({
  selector: 'app-settings',
  standalone: true,
  imports: [
    CommonModule,
    NzCardModule,
    NzFormModule,
    NzInputModule,
    NzSwitchModule,
    NzButtonModule,
    NzSelectModule,
    FormsModule
  ],
  templateUrl: './settings.component.html',
  styleUrls: ['./settings.component.scss']
})
export class SettingsComponent {
  settings = {
    siteName: '管理系统',
    siteDescription: '基于Angular和ng-zorro-antd的管理系统',
    enableNotifications: true,
    enableAuditLog: true,
    sessionTimeout: 30,
    language: 'zh_CN'
  };

  languages = [
    { value: 'zh_CN', label: '简体中文' },
    { value: 'en_US', label: 'English' }
  ];

  saveSettings(): void {
    // 这里可以添加保存设置的逻辑
    console.log('保存设置:', this.settings);
  }
}
