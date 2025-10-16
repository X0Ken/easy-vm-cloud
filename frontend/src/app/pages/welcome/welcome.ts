import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzIconModule } from 'ng-zorro-antd/icon';
import { NzMessageService } from 'ng-zorro-antd/message';
import { AuthService } from '../../services/auth.service';

@Component({
  selector: 'app-welcome',
  standalone: true,
  imports: [CommonModule, NzButtonModule, NzCardModule, NzIconModule, RouterModule],
  templateUrl: './welcome.html',
  styleUrl: './welcome.scss'
})
export class Welcome {
  constructor(
    private authService: AuthService,
    private router: Router,
    private message: NzMessageService
  ) {}

  logout(): void {
    this.authService.logout();
    this.message.success('已退出登录');
    this.router.navigate(['/login']);
  }
}
