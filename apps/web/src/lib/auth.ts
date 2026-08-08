export type AuthUser = {
  username: string;
  passwordChangeRequired: boolean;
};

export type AuthSession = {
  authenticated: boolean;
  user: AuthUser | null;
  csrfToken: string | null;
};

export type AuthClient = {
  session: () => Promise<AuthSession>;
  login: (username: string, password: string) => Promise<AuthSession>;
  logout: () => Promise<AuthSession>;
  changePassword: (currentPassword: string, newPassword: string) => Promise<void>;
  current: () => AuthSession;
};

type Fetcher = typeof fetch;

type SessionPayload = {
  user: AuthUser;
  csrfToken: string;
};

const anonymousSession: AuthSession = {
  authenticated: false,
  user: null,
  csrfToken: null
};

export function createAuthClient(fetcher: Fetcher = fetch): AuthClient {
  let currentSession = anonymousSession;

  async function session(): Promise<AuthSession> {
    const response = await fetcher('/api/auth/session', {
      credentials: 'include',
      headers: {
        accept: 'application/json'
      }
    });

    if (response.status === 401) {
      currentSession = anonymousSession;
      return currentSession;
    }

    if (!response.ok) {
      throw new Error('Unable to load authentication session');
    }

    currentSession = authenticatedSession(await readSessionPayload(response));
    return currentSession;
  }

  async function login(username: string, password: string): Promise<AuthSession> {
    const response = await fetcher('/api/auth/login', {
      method: 'POST',
      credentials: 'include',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json'
      },
      body: JSON.stringify({ username, password })
    });

    if (!response.ok) {
      currentSession = anonymousSession;
      throw new Error('Invalid username or password');
    }

    currentSession = authenticatedSession(await readSessionPayload(response));
    return currentSession;
  }

  async function logout(): Promise<AuthSession> {
    const csrfToken = currentSession.csrfToken;

    if (!csrfToken) {
      currentSession = anonymousSession;
      return currentSession;
    }

    const response = await fetcher('/api/auth/logout', {
      method: 'POST',
      credentials: 'include',
      headers: {
        'x-csrf-token': csrfToken
      }
    });

    if (!response.ok && response.status !== 401) {
      throw new Error('Unable to logout');
    }

    currentSession = anonymousSession;
    return currentSession;
  }

  async function changePassword(currentPassword: string, newPassword: string): Promise<void> {
    const csrfToken = currentSession.csrfToken;

    if (!csrfToken) {
      throw new Error('Authentication required');
    }

    const response = await fetcher('/api/auth/change-password', {
      method: 'POST',
      credentials: 'include',
      headers: {
        'content-type': 'application/json',
        'x-csrf-token': csrfToken
      },
      body: JSON.stringify({ currentPassword, newPassword })
    });

    if (!response.ok) {
      throw new Error('Unable to change password');
    }

    await session();
  }

  return {
    session,
    login,
    logout,
    changePassword,
    current: () => currentSession
  };
}

function authenticatedSession(payload: SessionPayload): AuthSession {
  return {
    authenticated: true,
    user: payload.user,
    csrfToken: payload.csrfToken
  };
}

async function readSessionPayload(response: Response): Promise<SessionPayload> {
  const payload = (await response.json()) as SessionPayload;

  if (
    typeof payload.csrfToken !== 'string' ||
    payload.csrfToken.length < 32 ||
    !payload.user ||
    typeof payload.user.username !== 'string' ||
    typeof payload.user.passwordChangeRequired !== 'boolean'
  ) {
    throw new Error('Invalid authentication response');
  }

  return payload;
}
