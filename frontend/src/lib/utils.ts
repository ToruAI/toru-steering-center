import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m ${secs}s`;
  }
  if (minutes > 0) {
    return `${minutes}m ${secs}s`;
  }
  return `${secs}s`;
}

export function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${size.toFixed(2)} ${units[unitIndex]}`;
}

export function generateStrongPassword(): string {
  const charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+";
  let password = "";
  // Ensure at least one of each required type
  password += "ABCDEFGHIJKLMNOPQRSTUVWXYZ".charAt(Math.floor(Math.random() * 26));
  password += "abcdefghijklmnopqrstuvwxyz".charAt(Math.floor(Math.random() * 26));
  password += "0123456789".charAt(Math.floor(Math.random() * 10));
  password += "!@#$%^&*()_+".charAt(Math.floor(Math.random() * 12));
  
  // Fill the rest
  for (let i = 4; i < 16; i++) {
    password += charset.charAt(Math.floor(Math.random() * charset.length));
  }
  
  // Shuffle
  return password.split('').sort(() => 0.5 - Math.random()).join('');
}

export function validatePasswordStrength(password: string): { valid: boolean; message?: string } {
  if (password.length < 8) return { valid: false, message: "Too short (min 8 chars)" };
  if (!/[A-Z]/.test(password)) return { valid: false, message: "Missing uppercase letter" };
  if (!/[a-z]/.test(password)) return { valid: false, message: "Missing lowercase letter" };
  if (!/[0-9]/.test(password)) return { valid: false, message: "Missing number" };
  if (!/[^A-Za-z0-9]/.test(password)) return { valid: false, message: "Missing special character" };
  return { valid: true };
}
