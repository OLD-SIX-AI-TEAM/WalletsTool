const fs = require('fs')
const path = require('path')

function removeWhitespace(value) {
  return String(value || '').replace(/\s+/g, '')
}

function isProbablyBase64(value) {
  if (!value) return false
  const v = removeWhitespace(value)
  if (v.length < 16) return false
  if (v.length % 4 !== 0) return false
  return /^[A-Za-z0-9+/]+={0,2}$/.test(v)
}

function tryBase64DecodeToUtf8(value) {
  try {
    const buf = Buffer.from(removeWhitespace(value), 'base64')
    const text = buf.toString('utf8')
    if (!text) return null
    if (text.includes('\uFFFD')) return null
    return text
  } catch {
    return null
  }
}

function looksLikeRsignKey(text) {
  if (!text) return false
  return (
    text.includes('untrusted comment:') &&
    (text.includes('rsign') || text.includes('minisign')) &&
    (text.includes('secret key') || text.includes('public key'))
  )
}

function writeGithubEnv(name, value) {
  const envFile = process.env.GITHUB_ENV
  if (!envFile) return
  fs.appendFileSync(envFile, `${name}=${value}\n`, { encoding: 'utf8' })
}

function main() {
  const rawKeyInput = process.env.TAURI_SIGNING_PRIVATE_KEY || ''
  const signingPwd = process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD || ''

  if (!rawKeyInput.trim()) {
    console.error('Missing TAURI_SIGNING_PRIVATE_KEY. Configure GitHub Actions secret: TAURI_SIGNING_PRIVATE_KEY.')
    process.exit(1)
  }

  if (signingPwd && signingPwd !== signingPwd.trim()) {
    console.warn('TAURI_SIGNING_PRIVATE_KEY_PASSWORD has leading/trailing whitespace. Remove whitespace in GitHub Secrets.')
  }

  const normalizedB64 = removeWhitespace(rawKeyInput)
  const keyContentIfB64 = isProbablyBase64(rawKeyInput) ? tryBase64DecodeToUtf8(rawKeyInput) : null
  const keyContent =
    keyContentIfB64 && looksLikeRsignKey(keyContentIfB64) ? keyContentIfB64 : String(rawKeyInput)

  if (keyContent.includes('minisign public key') || keyContent.includes('rsign public key')) {
    console.error('TAURI_SIGNING_PRIVATE_KEY looks like a public key. It must be the private key.')
    process.exit(1)
  }

  if (!looksLikeRsignKey(keyContent)) {
    console.error('TAURI_SIGNING_PRIVATE_KEY is not recognized as an rsign/minisign key. Ensure it is the updater signing private key.')
    process.exit(1)
  }

  const keyBase64 = keyContentIfB64 && looksLikeRsignKey(keyContentIfB64) ? normalizedB64 : Buffer.from(keyContent, 'utf8').toString('base64')

  const runnerTemp = process.env.RUNNER_TEMP || process.cwd()
  const keyPath = path.join(runnerTemp, 'tauri-updater-signing.key.b64')
  fs.writeFileSync(keyPath, keyBase64, { encoding: 'utf8' })

  writeGithubEnv('TAURI_SIGNING_PRIVATE_KEY', keyPath)
  console.log('Prepared TAURI_SIGNING_PRIVATE_KEY as a file path for tauri-action.')
}

main()
