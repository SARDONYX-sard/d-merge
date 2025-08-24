import { join } from 'node:path';
import { CallbackResponse, OnBeforeRequestListenerDetails } from 'electron';

export const handleAccessRequest = (
  details: OnBeforeRequestListenerDetails,
  callback: (response: CallbackResponse) => void,
) => {
  const callUrl = getActualURL(details.url);
  if (callUrl !== details.url) {
    callback({ redirectURL: callUrl });
  } else {
    callback({});
  }
};

/**
 * Intent: Correct paths such as `D:/frontend/_next`.
 */
const FRONTEND_REGEX = /[\w]:[/\\]frontend(\/.*)?$/i;

/**
 * Convert the URL `file:///.../frontend/...` to the local `__dirname/../frontend/...`.
 */
export const getActualURL = (originalUrl: string): string => {
  if (originalUrl.endsWith('/') || originalUrl.endsWith('frontend')) {
    return join(__dirname, '..', 'frontend', 'index.html');
  }

  const match = originalUrl.match(FRONTEND_REGEX);
  if (!match) return originalUrl;

  // match[1] = "/sub/path" or undefined
  let subPath = match[1] ?? '';
  if (subPath === '' || subPath === '/') {
    subPath = 'index.html';
  }

  return join(__dirname, '..', 'frontend', subPath);
};
