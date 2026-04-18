import { useState } from "react";
import { loginWithPassword } from "@/features/window/services/auth-api";
import { useAuthStore } from "@/features/window/stores/auth-store";
import { toast } from "@/ui/toast";

interface UseDesktopSignInOptions {
  onSuccess?: () => void;
}

export function useDesktopSignIn(options: UseDesktopSignInOptions = {}) {
  const refreshUser = useAuthStore((state) => state.refreshUser);
  const [isSigningIn, setIsSigningIn] = useState(false);

  const signIn = async () => {
    setIsSigningIn(true);

    try {
      const username = window.prompt("Username", "admin");
      if (!username) return;
      const password = window.prompt("Password");
      if (!password) return;
      await loginWithPassword(username, password);
      await refreshUser();
      toast.success("Signed in successfully!");
      options.onSuccess?.();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Authentication failed.";
      toast.error(message);

      throw error;
    } finally {
      setIsSigningIn(false);
    }
  };

  return {
    isSigningIn,
    signIn,
  };
}
