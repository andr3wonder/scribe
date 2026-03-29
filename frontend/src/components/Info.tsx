import React from "react";

interface InfoProps {
    isCollapsed: boolean;
}

const Info = React.forwardRef<HTMLDivElement, InfoProps>(({ isCollapsed }, ref) => {
  // Removed: About dialog and info button
  return null;
});

Info.displayName = "Info";

export default Info;