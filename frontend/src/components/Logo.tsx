import React from "react";

interface LogoProps {
    isCollapsed: boolean;
}

const Logo = React.forwardRef<HTMLDivElement, LogoProps>(({ isCollapsed }, ref) => {
  // Simplified: no branding, no dialog
  return null;
});

Logo.displayName = "Logo";

export default Logo;