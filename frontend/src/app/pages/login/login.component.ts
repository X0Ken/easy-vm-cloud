import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { NzFormModule } from 'ng-zorro-antd/form';
import { NzInputModule } from 'ng-zorro-antd/input';
import { NzButtonModule } from 'ng-zorro-antd/button';
import { NzCardModule } from 'ng-zorro-antd/card';
import { NzMessageService } from 'ng-zorro-antd/message';
import { AuthService } from '../../services/auth.service';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    NzFormModule,
    NzInputModule,
    NzButtonModule,
    NzCardModule
  ],
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.scss']
})
export class LoginComponent {
  username: string = '';
  password: string = '';
  loading: boolean = false;

  constructor(
    private authService: AuthService,
    private router: Router,
    private message: NzMessageService
  ) {}

  async onLogin(): Promise<void> {
    if (!this.username || !this.password) {
      this.message.error('请输入用户名和密码');
      return;
    }

    this.loading = true;
    try {
      const success = await this.authService.login(this.username, this.password);
      if (success) {
        this.message.success('登录成功');
        this.router.navigate(['/welcome']);
      } else {
        this.message.error('登录失败，请检查用户名和密码');
      }
    } catch (error) {
      this.message.error('登录失败，请稍后重试');
      console.error('登录错误:', error);
    } finally {
      this.loading = false;
    }
  }
}
