import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { AuthUser, SubscriptionInfo } from "@/features/window/services/auth-api";
import {
  buildCreationOptions,
  buildRequestOptions,
  changePasswordOnServer,
  fetchCurrentUser,
  fetchSubscriptionStatus,
  getAuthToken,
  loginWithPassword as loginWithPasswordApi,
  logoutFromServer,
  passkeyLoginFinish,
  passkeyLoginStart,
  passkeyRegisterFinish,
  passkeyRegisterStart,
  removeAuthToken,
  storeAuthToken,
} from "@/features/window/services/auth-api";

interface AuthState {
  user: AuthUser | null;
  subscription: SubscriptionInfo | null;
  isAuthenticated: boolean;
  forcePasswordChange: boolean;
  isLoading: boolean;
  error: string | null;
}

interface AuthActions {
  initialize: () => Promise<void>;
  loginWithPassword: (username: string, password: string) => Promise<void>;
  changePassword: (newPassword: string) => Promise<void>;
  loginWithPasskey: (username: string) => Promise<void>;
  registerPasskey: (name: string) => Promise<void>;
  handleAuthCallback: (token: string) => Promise<void>;
  refreshUser: () => Promise<void>;
  refreshSubscription: () => Promise<void>;
  logout: () => Promise<void>;
}

export const useAuthStore = create<AuthState & AuthActions>()(
  immer((set, get) => ({
    user: null,
    subscription: null,
    isAuthenticated: false,
    forcePasswordChange: false,
    isLoading: true,
    error: null,

    initialize: async () => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        await getAuthToken();
        const user = await fetchCurrentUser();
        const subscription = await fetchSubscriptionStatus();
        set((state) => {
          state.user = user;
          state.subscription = subscription;
          state.isAuthenticated = true;
          state.forcePasswordChange = user.forcePasswordChange;
          state.isLoading = false;
        });
      } catch {
        await removeAuthToken();
        set((state) => {
          state.user = null;
          state.subscription = null;
          state.isAuthenticated = false;
          state.forcePasswordChange = false;
          state.isLoading = false;
        });
      }
    },

    loginWithPassword: async (username: string, password: string) => {
      const { user, forcePasswordChange } = await loginWithPasswordApi(username, password);
      const subscription = await fetchSubscriptionStatus();
      set((state) => {
        state.user = user;
        state.subscription = subscription;
        state.isAuthenticated = true;
        state.forcePasswordChange = forcePasswordChange;
        state.error = null;
      });
    },

    changePassword: async (newPassword: string) => {
      await changePasswordOnServer(newPassword);
      // Re-fetch the user so forcePasswordChange is cleared
      const user = await fetchCurrentUser();
      set((state) => {
        state.user = user;
        state.forcePasswordChange = user.forcePasswordChange;
      });
    },

    loginWithPasskey: async (username: string) => {
      if (!window.PublicKeyCredential) {
        throw new Error("Passkeys are not supported in this browser.");
      }
      const start = await passkeyLoginStart(username);
      const credential = await navigator.credentials.get(buildRequestOptions(start));
      if (!credential) {
        throw new Error("Passkey sign-in was cancelled.");
      }
      await passkeyLoginFinish(start.challengeId, credential as PublicKeyCredential);
      // Session cookie is now set — fetch user to complete auth
      const user = await fetchCurrentUser();
      const subscription = await fetchSubscriptionStatus();
      set((state) => {
        state.user = user;
        state.subscription = subscription;
        state.isAuthenticated = true;
        state.forcePasswordChange = user.forcePasswordChange;
        state.error = null;
      });
    },

    registerPasskey: async (name: string) => {
      if (!window.PublicKeyCredential) {
        throw new Error("Passkeys are not supported in this browser.");
      }
      const start = await passkeyRegisterStart(name);
      const credential = await navigator.credentials.create(buildCreationOptions(start));
      if (!credential) {
        throw new Error("Passkey registration was cancelled.");
      }
      await passkeyRegisterFinish(start.challengeId, credential as PublicKeyCredential);
    },

    handleAuthCallback: async (token: string) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        await storeAuthToken(token);
        const user = await fetchCurrentUser();
        const subscription = await fetchSubscriptionStatus();
        set((state) => {
          state.user = user;
          state.subscription = subscription;
          state.isAuthenticated = true;
          state.forcePasswordChange = user.forcePasswordChange;
          state.isLoading = false;
        });
      } catch (error) {
        await removeAuthToken();
        set((state) => {
          state.user = null;
          state.subscription = null;
          state.isAuthenticated = false;
          state.forcePasswordChange = false;
          state.error = "Authentication failed. Please try again.";
          state.isLoading = false;
        });
        throw error;
      }
    },

    refreshUser: async () => {
      try {
        const user = await fetchCurrentUser();
        set((state) => {
          state.user = user;
          state.forcePasswordChange = user.forcePasswordChange;
        });
      } catch {
        await get().logout();
      }
    },

    refreshSubscription: async () => {
      try {
        const subscription = await fetchSubscriptionStatus();
        set((state) => {
          state.subscription = subscription;
        });
      } catch {
        // Ignore refresh failures
      }
    },

    logout: async () => {
      await logoutFromServer();
      await removeAuthToken();
      set((state) => {
        state.user = null;
        state.subscription = null;
        state.isAuthenticated = false;
        state.forcePasswordChange = false;
        state.error = null;
      });
    },
  })),
);
