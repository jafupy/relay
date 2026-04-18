import { fetch as relayFetch } from "@/lib/platform/http";
import { getApiBase } from "@/utils/api-base";

const API_BASE = getApiBase();

export interface AuthUser {
  id: string;
  username: string;
  displayName: string | null;
  role: "admin" | "user";
  forcePasswordChange: boolean;
  // compat fields for cloud auth consumers (normalized to sensible defaults)
  email?: string;
  name: string | null;
  avatar_url: string | null;
  provider: string | null;
  github_username: string | null;
  subscription_status: "free" | "pro";
  created_at: string;
}

export interface SubscriptionInfo {
  status: "free" | "pro";
  subscription: {
    plan: string;
    renews_at: string | null;
    ends_at: string | null;
  } | null;
  enterprise: {
    has_access: boolean;
    is_admin: boolean;
    policy: {
      managedMode: boolean;
      requireExtensionAllowlist: boolean;
      allowedExtensionIds: string[];
      allowByok: boolean;
      aiCompletionEnabled: boolean;
      aiChatEnabled: boolean;
      updatedAt: string | null;
    } | null;
  };
  autocomplete?: {
    usage?: Record<string, unknown> | null;
  } | null;
}

export interface EnterprisePolicy {
  managedMode: boolean;
  requireExtensionAllowlist: boolean;
  allowedExtensionIds: string[];
  allowByok: boolean;
  aiCompletionEnabled: boolean;
  aiChatEnabled: boolean;
  updatedAt: string | null;
}

function normalizeUser(user: AuthUser): AuthUser {
  return {
    ...user,
    email: user.email ?? user.username,
    name: user.name ?? user.displayName ?? user.username,
    avatar_url: user.avatar_url ?? null,
    provider: user.provider ?? "local",
    github_username: user.github_username ?? null,
    subscription_status: user.subscription_status ?? "free",
    created_at: user.created_at ?? "",
    forcePasswordChange: user.forcePasswordChange ?? false,
  };
}

async function sessionFetch(path: string, options: RequestInit = {}): Promise<Response> {
  return relayFetch(`${API_BASE}${path}`, {
    ...options,
    credentials: "same-origin",
    headers: {
      "Content-Type": "application/json",
      ...options.headers,
    },
  });
}

export const getAuthToken = async (): Promise<string | null> => null;

export const storeAuthToken = async (_token: string): Promise<void> => {};

export const removeAuthToken = async (): Promise<void> => {};

export interface LoginResult {
  user: AuthUser;
  forcePasswordChange: boolean;
}

export async function loginWithPassword(username: string, password: string): Promise<LoginResult> {
  const response = await sessionFetch("/api/auth/login", {
    method: "POST",
    body: JSON.stringify({ username, password }),
  });

  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error((data as { error?: string }).error ?? "Invalid username or password");
  }

  const data = await response.json();
  return {
    user: normalizeUser(data.user as AuthUser),
    forcePasswordChange: data.forcePasswordChange as boolean,
  };
}

export async function changePasswordOnServer(newPassword: string): Promise<void> {
  const response = await sessionFetch("/api/auth/password", {
    method: "POST",
    body: JSON.stringify({ newPassword }),
  });

  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error((data as { error?: string }).error ?? "Failed to change password");
  }
}

export async function fetchCurrentUser(): Promise<AuthUser> {
  const response = await sessionFetch("/api/auth/me");
  if (!response.ok) {
    throw new Error(`Unauthenticated: ${response.status}`);
  }
  const data = await response.json();
  return normalizeUser(data.user as AuthUser);
}

export async function fetchSubscriptionStatus(): Promise<SubscriptionInfo> {
  return {
    status: "free",
    subscription: null,
    enterprise: {
      has_access: false,
      is_admin: false,
      policy: null,
    },
    autocomplete: null,
  };
}

export async function updateEnterprisePolicy(
  _patch: Partial<Omit<EnterprisePolicy, "updatedAt">>,
): Promise<EnterprisePolicy> {
  throw new Error("Enterprise policy is not available on local Relay accounts.");
}

export async function logoutFromServer(): Promise<void> {
  await sessionFetch("/api/auth/logout", { method: "POST" }).catch(() => {});
}

// ── WebAuthn / Passkey helpers ──────────────────────────────────────────────

function b64ToBuf(value: string): ArrayBuffer {
  const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
  const binary = atob(padded);
  return Uint8Array.from(binary, (c) => c.charCodeAt(0)).buffer;
}

function bufToB64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

interface RawCreationOptions {
  challengeId: string;
  publicKey: {
    challenge: string;
    user: { id: string };
    excludeCredentials?: Array<{ id: string; [k: string]: unknown }>;
    [k: string]: unknown;
  };
}

interface RawRequestOptions {
  challengeId: string;
  mediation?: CredentialMediationRequirement;
  publicKey: {
    challenge: string;
    allowCredentials?: Array<{ id: string; [k: string]: unknown }>;
    [k: string]: unknown;
  };
}

export function buildCreationOptions(raw: RawCreationOptions): CredentialCreationOptions {
  const options = { ...raw.publicKey } as unknown as Record<string, unknown>;
  options.challenge = b64ToBuf(raw.publicKey.challenge);
  options.user = { ...raw.publicKey.user, id: b64ToBuf(raw.publicKey.user.id) };
  options.excludeCredentials = (raw.publicKey.excludeCredentials ?? []).map((item) => ({
    ...item,
    id: b64ToBuf(item.id),
  }));
  return { publicKey: options as unknown as PublicKeyCredentialCreationOptions };
}

export function buildRequestOptions(raw: RawRequestOptions): CredentialRequestOptions {
  const options = { ...raw.publicKey } as unknown as Record<string, unknown>;
  options.challenge = b64ToBuf(raw.publicKey.challenge);
  options.allowCredentials = (raw.publicKey.allowCredentials ?? []).map((item) => ({
    ...item,
    id: b64ToBuf(item.id),
  }));
  const result: CredentialRequestOptions = {
    publicKey: options as unknown as PublicKeyCredentialRequestOptions,
  };
  if (raw.mediation) result.mediation = raw.mediation;
  return result;
}

export function serializeRegistrationCredential(credential: PublicKeyCredential) {
  const response = credential.response as AuthenticatorAttestationResponse;
  return {
    id: credential.id,
    rawId: bufToB64(credential.rawId),
    type: credential.type,
    response: {
      attestationObject: bufToB64(response.attestationObject),
      clientDataJSON: bufToB64(response.clientDataJSON),
      transports: response.getTransports ? response.getTransports() : undefined,
    },
    extensions: credential.getClientExtensionResults(),
  };
}

export function serializeAuthCredential(credential: PublicKeyCredential) {
  const response = credential.response as AuthenticatorAssertionResponse;
  return {
    id: credential.id,
    rawId: bufToB64(credential.rawId),
    type: credential.type,
    response: {
      authenticatorData: bufToB64(response.authenticatorData),
      clientDataJSON: bufToB64(response.clientDataJSON),
      signature: bufToB64(response.signature),
      userHandle: response.userHandle ? bufToB64(response.userHandle) : null,
    },
    extensions: credential.getClientExtensionResults(),
  };
}

export async function passkeyLoginStart(username: string): Promise<RawRequestOptions> {
  const response = await sessionFetch("/api/auth/passkeys/login/start", {
    method: "POST",
    body: JSON.stringify({ username }),
  });
  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error((data as { error?: string }).error ?? "Failed to start passkey sign-in");
  }
  return response.json() as Promise<RawRequestOptions>;
}

export async function passkeyLoginFinish(
  challengeId: string,
  credential: PublicKeyCredential,
): Promise<void> {
  const response = await sessionFetch("/api/auth/passkeys/login/finish", {
    method: "POST",
    body: JSON.stringify({ challengeId, credential: serializeAuthCredential(credential) }),
  });
  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error((data as { error?: string }).error ?? "Passkey sign-in failed");
  }
}

export async function passkeyRegisterStart(name: string): Promise<RawCreationOptions> {
  const response = await sessionFetch("/api/auth/passkeys/register/start", {
    method: "POST",
    body: JSON.stringify({ name }),
  });
  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error((data as { error?: string }).error ?? "Failed to start passkey registration");
  }
  return response.json() as Promise<RawCreationOptions>;
}

export async function passkeyRegisterFinish(
  challengeId: string,
  credential: PublicKeyCredential,
): Promise<void> {
  const response = await sessionFetch("/api/auth/passkeys/register/finish", {
    method: "POST",
    body: JSON.stringify({
      challengeId,
      credential: serializeRegistrationCredential(credential),
    }),
  });
  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error((data as { error?: string }).error ?? "Passkey registration failed");
  }
}
