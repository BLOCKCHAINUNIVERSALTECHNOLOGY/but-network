package tech.bharatunified.butwallet

import android.os.Bundle
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

/**
 * MainActivity — BUT Wallet ki main screen (SKELETON / DEMO ONLY).
 *
 * Abhi sirf UI ka dhaancha hai:
 * - "Naya wallet banao" device key banata hai (Keystore, non-exportable).
 * - Asli BIP39 / Ed25519 wallet, Device Lock aur network abhi NAHI hai.
 * - Ye code abhi compile/test NAHI hua hai (is machine par SDK nahi hai).
 */
class MainActivity : AppCompatActivity() {

    private lateinit var statusText: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        statusText = findViewById(R.id.statusText)
        val createButton: Button = findViewById(R.id.createWalletButton)
        val signButton: Button = findViewById(R.id.signTestButton)

        refreshStatus()

        createButton.setOnClickListener {
            try {
                if (!WalletCrypto.hasDeviceKey()) {
                    WalletCrypto.generateDeviceKey()
                    Toast.makeText(this, "Device key ban gayi", Toast.LENGTH_SHORT).show()
                } else {
                    Toast.makeText(this, "Device key pehle se hai", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                Toast.makeText(this, "Key nahi bani: ${e.message}", Toast.LENGTH_LONG).show()
            }
            refreshStatus()
        }

        signButton.setOnClickListener {
            try {
                // Sirf test: "BUT-test" ko sign karo. Asli transaction yahan nahi hai.
                val signature = WalletCrypto.sign("BUT-test".toByteArray(Charsets.UTF_8))
                statusText.text = "Sign ho gaya. Signature length: ${signature.size} bytes"
            } catch (e: Exception) {
                Toast.makeText(this, "Sign nahi hua: ${e.message}", Toast.LENGTH_LONG).show()
            }
        }
    }

    private fun refreshStatus() {
        statusText.text = if (WalletCrypto.hasDeviceKey()) {
            "Device key: maujood hai (Keystore me band hai)"
        } else {
            "Device key: abhi nahi bani"
        }
    }
}
