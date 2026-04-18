import { Button, type ButtonProps } from "@/ui/button";

interface ProActionButtonProps extends Omit<ButtonProps, "onClick"> {
  onProClick: () => void;
}

export function ProActionButton({ onProClick, children, ...props }: ProActionButtonProps) {
  return (
    <Button {...props} onClick={onProClick}>
      {children}
    </Button>
  );
}
