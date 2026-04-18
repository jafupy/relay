import type { ReactNode } from "react";

interface ProGateProps {
  children: ReactNode;
  fallback?: ReactNode;
}

export function ProGate({ children, fallback }: ProGateProps) {
  return <>{children ?? fallback}</>;
}
