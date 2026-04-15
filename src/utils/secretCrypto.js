function bytesToHex(bytes) {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

function hexToBytes(hex) {
  const s = (hex || '').trim();
  if (!s || s.length % 2 !== 0) throw new Error('hex 格式错误');
  const out = new Uint8Array(s.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(s.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

function bytesToBase64(bytes) {
  let binary = '';
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
  }
  return btoa(binary);
}

function base64ToBytes(b64) {
  const binary = atob(b64);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i);
  return out;
}

async function deriveAesKey(password, saltBytes, iterations = 100000) {
  const enc = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey('raw', enc.encode(password), 'PBKDF2', false, ['deriveKey']);
  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: saltBytes,
      iterations,
      hash: 'SHA-256'
    },
    keyMaterial,
    { name: 'AES-CBC', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
}

export async function sealSecret(plaintext, password) {
  const text = String(plaintext ?? '').trim();
  if (!text) throw new Error('密文内容为空');
  const pwd = String(password ?? '');
  if (!pwd) throw new Error('缺少密码');

  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(16));
  const key = await deriveAesKey(pwd, salt);
  const enc = new TextEncoder();
  const ciphertext = new Uint8Array(
    await crypto.subtle.encrypt({ name: 'AES-CBC', iv }, key, enc.encode(text))
  );

  return `p1:${bytesToHex(salt)}:${bytesToHex(iv)}:${bytesToBase64(ciphertext)}`;
}

export async function openSealedSecret(sealed, password) {
  const s = String(sealed ?? '').trim();
  const pwd = String(password ?? '');
  if (!pwd) throw new Error('缺少密码');
  if (!s) throw new Error('密文为空');
  if (!s.startsWith('p1:')) throw new Error('必须使用加密格式传输');

  const parts = s.slice(3).split(':');
  if (parts.length !== 3) throw new Error('密文格式错误');
  const [saltHex, ivHex, cipherB64] = parts;
  const salt = hexToBytes(saltHex);
  const iv = hexToBytes(ivHex);
  const cipher = base64ToBytes(cipherB64);
  const key = await deriveAesKey(pwd, salt);
  const plaintextBytes = new Uint8Array(
    await crypto.subtle.decrypt({ name: 'AES-CBC', iv }, key, cipher)
  );
  const dec = new TextDecoder();
  return dec.decode(plaintextBytes);
}

