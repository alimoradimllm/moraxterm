export function detectAuthCapabilities(environment = globalThis) {
  const navigatorValue = environment.navigator;
  const publicKey = environment.PublicKeyCredential;
  const webAuthn = Boolean(navigatorValue?.credentials && publicKey);
  const platform = String(navigatorValue?.platform || '').toLowerCase();
  return {
    webAuthn,
    windowsHello: false,
    fido2Ssh: false,
    status: webAuthn
      ? 'WebAuthn is exposed by the host, but Moraxterm has no platform authentication adapter.'
      : 'Biometric and FIDO2 authentication is unsupported in this build.',
    platform: platform.includes('win') ? 'windows' : platform || 'unknown',
  };
}
