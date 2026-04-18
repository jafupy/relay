import { lazy, Suspense } from "react";
import { useAppBootstrap } from "@/bootstrap/use-app-bootstrap";
import { useOnboardingStore } from "@/features/onboarding/store";
import { FontStyleInjector } from "@/features/settings/components/font-style-injector";
import { useAutoUpdate } from "@/features/settings/hooks/use-auto-update";
import { LoginPage } from "@/features/window/components/login-page";
import { useAuthStore } from "@/features/window/stores/auth-store";

const OnboardingDialog = lazy(() => import("@/features/onboarding/components/onboarding-dialog"));
const UpdateDialog = lazy(() => import("@/features/settings/components/update-dialog"));

import { MainLayout } from "./features/layout/components/main-layout";
import { WindowResizeBorder } from "./features/window/components/window-resize-border";
import { ZoomIndicator } from "./features/window/components/zoom-indicator";
import { ToastContainer } from "./ui/toast";

function App() {
  useAppBootstrap();

  const isLoading = useAuthStore((state) => state.isLoading);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const forcePasswordChange = useAuthStore((state) => state.forcePasswordChange);

  // Auto-update check
  const {
    showDialog: showUpdateDialog,
    updateInfo,
    downloadProgress,
    downloading,
    installing,
    error: updateError,
    onDismiss: dismissUpdate,
    onDownload: downloadUpdate,
  } = useAutoUpdate();
  const isOnboardingOpen = useOnboardingStore((state) => state.isOpen);
  const onboardingContext = useOnboardingStore((state) => state.context);
  const dismissOnboarding = useOnboardingStore((state) => state.dismiss);
  const completeOnboarding = useOnboardingStore((state) => state.complete);

  // While auth state is being resolved show a blank dark screen
  if (isLoading) {
    return <div className="h-dvh w-dvw bg-secondary-bg" />;
  }

  // Not authenticated, or authenticated but must change password
  if (!isAuthenticated || forcePasswordChange) {
    return (
      <div className="h-dvh w-dvw overflow-hidden">
        <FontStyleInjector />
        <WindowResizeBorder />
        <LoginPage />
      </div>
    );
  }

  return (
    <>
      {/* Linux window resize handles (must be outside zoom container) */}
      <WindowResizeBorder />

      <div className="h-dvh w-dvw overflow-hidden">
        <FontStyleInjector />
        <div className="window-container flex h-full w-full flex-col overflow-hidden bg-primary-bg">
          <MainLayout />
        </div>
        <ZoomIndicator />
        <ToastContainer />

        {showUpdateDialog && updateInfo && (
          <Suspense fallback={null}>
            <UpdateDialog
              updateInfo={updateInfo}
              downloadProgress={downloadProgress}
              downloading={downloading}
              installing={installing}
              error={updateError}
              onDownload={downloadUpdate}
              onDismiss={dismissUpdate}
            />
          </Suspense>
        )}

        {isOnboardingOpen && onboardingContext && (
          <Suspense fallback={null}>
            <OnboardingDialog
              context={onboardingContext}
              onClose={() => void dismissOnboarding()}
              onComplete={completeOnboarding}
            />
          </Suspense>
        )}
      </div>
    </>
  );
}

export default App;
