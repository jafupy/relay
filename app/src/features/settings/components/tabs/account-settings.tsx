import { LogIn } from "lucide-react";
import { useDesktopSignIn } from "@/features/window/hooks/use-desktop-sign-in";
import { useAuthStore } from "@/features/window/stores/auth-store";
import { Button } from "@/ui/button";
import Section, { SettingRow } from "../settings-section";

export const AccountSettings = () => {
  const user = useAuthStore((state) => state.user);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const { isSigningIn, signIn } = useDesktopSignIn();

  return (
    <div className="space-y-4">
      <Section title="Account">
        <SettingRow
          label={isAuthenticated ? "Signed In" : "Sign In"}
          description={
            isAuthenticated
              ? "Your current account session."
              : "Sign in with a local Relay account."
          }
        >
          {isAuthenticated ? (
            <span className="ui-font text-[length:var(--app-ui-control-font-size)] text-text-lighter">
              {user?.email}
            </span>
          ) : (
            <Button
              variant="secondary"
              size="xs"
              onClick={signIn}
              disabled={isSigningIn}
              className="ui-text-sm"
            >
              <LogIn />
              {isSigningIn ? "Signing In..." : "Sign In"}
            </Button>
          )}
        </SettingRow>
      </Section>
    </div>
  );
};
