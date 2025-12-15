interface ToruLogoProps {
  className?: string;
  size?: 'sm' | 'md' | 'lg';
  showText?: boolean;
}

export function ToruLogo({ className = "", size = 'md', showText = true }: ToruLogoProps) {
  const sizeMap = {
    sm: 'w-8 h-8',
    md: 'w-12 h-12',
    lg: 'w-16 h-16'
  };
  
  const textSizeMap = {
    sm: 'text-sm',
    md: 'text-base',
    lg: 'text-lg'
  };

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      <img 
        src="/favicon.png" 
        alt="Steering Center Logo" 
        className={`${sizeMap[size]} object-contain`}
      />
      {showText && (
        <span className={`font-bold text-toru-text-primary ${textSizeMap[size]}`}>
          Steering Center
        </span>
      )}
    </div>
  );
}
