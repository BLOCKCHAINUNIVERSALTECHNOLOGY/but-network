package tech.bharatunified.butwallet

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.Signature

/**
 * WalletCrypto — BUT Wallet ka security hissa (SKELETON / DEMO ONLY).
 *
 * Design (docs/DEVICE-LOCK-DESIGN.md dekho):
 * - Device key Android Keystore me banti hai aur kabhi bahar NAHI niklegi
 *   (non-exportable). Sign hardware ke andar hota hai.
 * - Baaki data EncryptedSharedPreferences me rehta hai (AES-256-GCM),
 *   jiski key bhi Keystore me hai.
 * - Ye code abhi compile/test NAHI hua hai (is machine par SDK nahi hai).
 * - Isme asli paisa kabhi mat rakho. Production se pehle audit chahiye.
 */
object WalletCrypto {

    /** Keystore me device key ka naam. */
    const val KEYSTORE_ALIAS = "but-wallet-device-key"

    /** Encrypted prefs file ka naam. */
    const val PREFS_FILE = "but-wallet-prefs"

    /** Master key — AES256_GCM scheme (Keystore-backed). */
    fun masterKey(context: Context): MasterKey =
        MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()

    /** Encrypted SharedPreferences — key aur value dono encrypt hote hain. */
    fun encryptedPrefs(context: Context): SharedPreferences =
        EncryptedSharedPreferences.create(
            context,
            PREFS_FILE,
            masterKey(context),
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )

    /**
     * Device-bound EC keypair banao.
     * - Key Keystore ke andar banti hai, export NAHI hoti.
     * - Har use par PIN / biometric chahiye (setUserAuthenticationRequired).
     * - Naya biometric add hote hi key invalid ho jati hai.
     */
    fun generateDeviceKey() {
        val keyPairGenerator = KeyPairGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_EC,
            "AndroidKeyStore",
        )
        val spec = KeyGenParameterSpec.Builder(
            KEYSTORE_ALIAS,
            KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY,
        )
            .setDigests(KeyProperties.DIGEST_SHA256)
            .setUserAuthenticationRequired(true)
            .setInvalidatedByBiometricEnrollment(true)
            .build()
        keyPairGenerator.initialize(spec)
        keyPairGenerator.generateKeyPair()
    }

    /** Kya device key pehle se bani hui hai? */
    fun hasDeviceKey(): Boolean {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        return keyStore.containsAlias(KEYSTORE_ALIAS)
    }

    /**
     * Device key se sign karo. Private key kabhi memory me plain nahi aati —
     * sign Keystore ke andar hota hai.
     */
    fun sign(data: ByteArray): ByteArray {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val privateKey = keyStore.getKey(KEYSTORE_ALIAS, null) as PrivateKey
        val signature = Signature.getInstance("SHA256withECDSA")
        signature.initSign(privateKey)
        signature.update(data)
        return signature.sign()
    }
}
