import type { DocsFrontend } from './docs'

const FRONTEND_COOKIE = 'quickgui-frontend'

export function readFrontendPreference(cookieHeader: string | null): DocsFrontend {
  const value = cookieHeader
    ?.split(';')
    .map((cookie) => cookie.trim())
    .find((cookie) => cookie.startsWith(`${FRONTEND_COOKIE}=`))
    ?.slice(FRONTEND_COOKIE.length + 1)

  return value === 'typescript' || value === 'rust' ? value : 'go'
}

export function rememberFrontend(frontend: DocsFrontend): void {
  try {
    document.cookie = `${FRONTEND_COOKIE}=${frontend}; Path=/; Max-Age=31536000; SameSite=Lax`
  } catch {
    // Keep the picker usable when the browser blocks cookies.
  }
}
