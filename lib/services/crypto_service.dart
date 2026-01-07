import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';
import 'package:pointycastle/export.dart';

/// AES-256-GCM 加密服务
/// 使用 Argon2 派生密钥（这里用 PBKDF2 替代，因为 pointycastle 支持更好）
class CryptoService {
  /// GCM nonce 长度（12 字节 = 96 位）
  static const int nonceLength = 12;

  /// Salt 长度（16 字节 = 128 位）
  static const int saltLength = 16;

  /// GCM tag 长度（16 字节 = 128 位）
  static const int tagLength = 16;

  /// 密钥长度（32 字节 = 256 位）
  static const int keyLength = 32;

  /// PBKDF2 迭代次数
  static const int pbkdf2Iterations = 100000;

  /// 生成随机字节
  static Uint8List generateRandomBytes(int length) {
    final random = Random.secure();
    return Uint8List.fromList(
      List.generate(length, (_) => random.nextInt(256)),
    );
  }

  /// 从密码派生密钥（使用 PBKDF2-HMAC-SHA256）
  static Uint8List deriveKey(String password, Uint8List salt) {
    final pbkdf2 = PBKDF2KeyDerivator(HMac(SHA256Digest(), 64))
      ..init(Pbkdf2Parameters(salt, pbkdf2Iterations, keyLength));

    return pbkdf2.process(Uint8List.fromList(utf8.encode(password)));
  }

  /// AES-256-GCM 加密
  /// 返回格式：salt (16) + nonce (12) + ciphertext + tag (16)
  static Uint8List encrypt(Uint8List plaintext, String password) {
    // 生成随机 salt 和 nonce
    final salt = generateRandomBytes(saltLength);
    final nonce = generateRandomBytes(nonceLength);

    // 从密码派生密钥
    final key = deriveKey(password, salt);

    // 初始化 AES-GCM
    final cipher = GCMBlockCipher(AESEngine())
      ..init(
        true, // 加密模式
        AEADParameters(
          KeyParameter(key),
          tagLength * 8, // tag 长度（位）
          nonce,
          Uint8List(0), // 无附加认证数据
        ),
      );

    // 加密
    final ciphertext = Uint8List(cipher.getOutputSize(plaintext.length));
    final len =
        cipher.processBytes(plaintext, 0, plaintext.length, ciphertext, 0);
    cipher.doFinal(ciphertext, len);

    // 组合结果：salt + nonce + ciphertext（包含 tag）
    final result = BytesBuilder();
    result.add(salt);
    result.add(nonce);
    result.add(ciphertext);

    return result.toBytes();
  }

  /// AES-256-GCM 解密
  /// 输入格式：salt (16) + nonce (12) + ciphertext + tag (16)
  static Uint8List decrypt(Uint8List ciphertext, String password) {
    if (ciphertext.length < saltLength + nonceLength + tagLength) {
      throw ArgumentError('密文太短');
    }

    // 提取 salt 和 nonce
    final salt = ciphertext.sublist(0, saltLength);
    final nonce = ciphertext.sublist(saltLength, saltLength + nonceLength);
    final encryptedData = ciphertext.sublist(saltLength + nonceLength);

    // 从密码派生密钥
    final key = deriveKey(password, Uint8List.fromList(salt));

    // 初始化 AES-GCM
    final cipher = GCMBlockCipher(AESEngine())
      ..init(
        false, // 解密模式
        AEADParameters(
          KeyParameter(key),
          tagLength * 8, // tag 长度（位）
          Uint8List.fromList(nonce),
          Uint8List(0), // 无附加认证数据
        ),
      );

    // 解密
    final plaintext = Uint8List(cipher.getOutputSize(encryptedData.length));
    final len = cipher.processBytes(
        encryptedData, 0, encryptedData.length, plaintext, 0);
    cipher.doFinal(plaintext, len);

    return plaintext;
  }

  /// 验证密码是否正确
  /// 尝试解密并检查是否成功
  static bool verifyPassword(Uint8List ciphertext, String password) {
    try {
      decrypt(ciphertext, password);
      return true;
    } catch (e) {
      return false;
    }
  }

  /// 加密字符串
  static String encryptString(String plaintext, String password) {
    final encrypted =
        encrypt(Uint8List.fromList(utf8.encode(plaintext)), password);
    return base64Encode(encrypted);
  }

  /// 解密字符串
  static String decryptString(String ciphertext, String password) {
    final decrypted =
        decrypt(Uint8List.fromList(base64Decode(ciphertext)), password);
    return utf8.decode(decrypted);
  }
}
