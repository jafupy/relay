import { useState } from "react";
import { useAuthStore } from "@/features/window/stores/auth-store";
import { cn } from "@/utils/cn";

export function LoginPage() {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [passkeyName, setPasskeyName] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [busy, setBusy] = useState(false);
  const [passkeyRegistered, setPasskeyRegistered] = useState(false);

  const {
    loginWithPassword,
    changePassword,
    loginWithPasskey,
    registerPasskey,
    forcePasswordChange,
  } = useAuthStore();

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setSuccess("");
    setBusy(true);
    try {
      await loginWithPassword(username, password);
      // If forcePasswordChange, the store updates and App re-renders to show the change-password screen
    } catch (err) {
      setError(err instanceof Error ? err.message : "Sign in failed");
    } finally {
      setBusy(false);
    }
  };

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setSuccess("");
    setBusy(true);
    try {
      await changePassword(newPassword);
      setSuccess("Password changed. You're signed in.");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to change password");
    } finally {
      setBusy(false);
    }
  };

  const handlePasskeyLogin = async () => {
    setError("");
    setSuccess("");
    setBusy(true);
    try {
      await loginWithPasskey(username);
    } catch (err) {
      if (err instanceof Error && err.name === "NotAllowedError") {
        setError("Passkey sign-in was cancelled or timed out.");
      } else {
        setError(err instanceof Error ? err.message : "Passkey sign-in failed");
      }
    } finally {
      setBusy(false);
    }
  };

  const handleRegisterPasskey = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setSuccess("");
    setBusy(true);
    try {
      await registerPasskey(passkeyName || navigator.userAgent.slice(0, 80));
      setPasskeyRegistered(true);
      setSuccess("Passkey registered successfully.");
    } catch (err) {
      if (err instanceof Error && err.name === "NotAllowedError") {
        setError("Passkey registration was cancelled or timed out.");
      } else {
        setError(err instanceof Error ? err.message : "Passkey registration failed");
      }
    } finally {
      setBusy(false);
    }
  };

  const showChangePassword = forcePasswordChange;

  return (
    <div className="flex min-h-dvh w-dvw items-center justify-center bg-secondary-bg">
      <div className="w-full max-w-sm rounded-xl border border-border/60 bg-primary-bg p-9 shadow-2xl shadow-black/40">
        {/* Heading */}
        <h1
          className="mb-1 text-[38px] font-semibold leading-none tracking-tight text-text"
          style={{ fontFamily: "var(--font-serif)" }}
        >
          Relay
        </h1>
        <p className="mb-7 text-[13px] text-text-lighter">
          {showChangePassword
            ? "Set a new password before continuing."
            : "Sign in with a local account."}
        </p>

        {/* Login form */}
        {!showChangePassword && (
          <>
            <form onSubmit={handleLogin} className="flex flex-col gap-3.5">
              <Field label="Username">
                <input
                  className={inputClass}
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  autoComplete="username"
                  autoFocus
                  disabled={busy}
                />
              </Field>
              <Field label="Password">
                <input
                  className={inputClass}
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  autoComplete="current-password"
                  disabled={busy}
                />
              </Field>
              <PrimaryButton type="submit" disabled={busy}>
                {busy ? "Signing in…" : "Sign In"}
              </PrimaryButton>
            </form>

            <Divider />

            <SecondaryButton type="button" onClick={handlePasskeyLogin} disabled={busy}>
              Sign In With Passkey
            </SecondaryButton>
          </>
        )}

        {/* Force change password form */}
        {showChangePassword && (
          <>
            <form onSubmit={handleChangePassword} className="flex flex-col gap-3.5">
              <Field label="New password">
                <input
                  className={inputClass}
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  autoComplete="new-password"
                  autoFocus
                  disabled={busy}
                  minLength={12}
                />
              </Field>
              <PrimaryButton type="submit" disabled={busy || passkeyRegistered}>
                {busy ? "Changing…" : "Change Password"}
              </PrimaryButton>
            </form>

            {success && !passkeyRegistered && (
              <>
                <Divider />
                <form onSubmit={handleRegisterPasskey} className="flex flex-col gap-3.5">
                  <Field label="Passkey name (optional)">
                    <input
                      className={inputClass}
                      value={passkeyName}
                      onChange={(e) => setPasskeyName(e.target.value)}
                      placeholder="e.g. MacBook Touch ID"
                      disabled={busy}
                    />
                  </Field>
                  <SecondaryButton type="submit" disabled={busy}>
                    Register Passkey
                  </SecondaryButton>
                </form>
              </>
            )}
          </>
        )}

        {/* Feedback */}
        {error && <p className="mt-4 text-[13px] text-red-400">{error}</p>}
        {success && <p className="mt-4 text-[13px] text-emerald-400">{success}</p>}
      </div>
    </div>
  );
}

// ── Small sub-components ──────────────────────────────────────────────────────

const inputClass =
  "w-full rounded-md border border-border bg-hover px-3 py-2 text-[14px] text-text outline-none transition-[border-color,box-shadow] placeholder:text-text-lighter focus:border-accent focus:shadow-[0_0_0_3px_color-mix(in_srgb,var(--accent)_15%,transparent)] disabled:opacity-50";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-[11px] font-semibold uppercase tracking-widest text-text-lighter">
        {label}
      </span>
      {children}
    </label>
  );
}

function PrimaryButton({
  children,
  disabled,
  type,
}: {
  children: React.ReactNode;
  disabled?: boolean;
  type?: "submit" | "button";
}) {
  return (
    <button
      type={type ?? "button"}
      disabled={disabled}
      className={cn(
        "mt-1 w-full rounded-md bg-accent px-4 py-2.5 text-[14px] font-medium text-white transition-opacity",
        "hover:opacity-85 active:opacity-75 disabled:cursor-not-allowed disabled:opacity-50",
      )}
    >
      {children}
    </button>
  );
}

function SecondaryButton({
  children,
  disabled,
  type,
  onClick,
}: {
  children: React.ReactNode;
  disabled?: boolean;
  type?: "submit" | "button";
  onClick?: () => void;
}) {
  return (
    <button
      type={type ?? "button"}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "w-full rounded-md border border-border px-4 py-2 text-[13px] font-medium text-text-lighter transition-[border-color,color,background-color]",
        "hover:border-border/80 hover:bg-hover hover:text-text disabled:cursor-not-allowed disabled:opacity-50",
      )}
    >
      {children}
    </button>
  );
}

function Divider() {
  return (
    <div className="my-5 flex items-center gap-2.5 text-[11px] text-text-lighter">
      <span className="h-px flex-1 bg-border/60" />
      or
      <span className="h-px flex-1 bg-border/60" />
    </div>
  );
}
