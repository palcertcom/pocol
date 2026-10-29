package org.pocol.node;
import android.Manifest;
import android.app.*;
import android.content.*;
import android.content.pm.PackageManager;
import android.os.*;
import android.view.*;
import android.widget.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import org.json.*;

public final class MainActivity extends Activity {
    private TextView status, identity;
    private final Handler main = new Handler(Looper.getMainLooper());
    private Messenger service;
    private boolean bound;
    private String home, address;
    private final Messenger receiver = new Messenger(new Handler(Looper.getMainLooper(), msg -> {
        if (msg.what == 1) showStatus(msg.getData().getString("status", "{}")); return true;
    }));
    private final ServiceConnection connection = new ServiceConnection() {
        public void onServiceConnected(ComponentName name, IBinder binder) { service = new Messenger(binder); }
        public void onServiceDisconnected(ComponentName name) { service = null; }
    };
    private final Runnable refresh = new Runnable() {
        public void run() {
            if (service != null) {
                Message request = Message.obtain(null, 1); request.replyTo = receiver;
                try { service.send(request); } catch (RemoteException e) { service = null; }
            } else {
                File result = new File(getFilesDir(), "last-result.json");
                if (result.isFile()) try (InputStream in = new FileInputStream(result)) { showStatus(new String(readLimited(in), StandardCharsets.UTF_8)); } catch (IOException ignored) {}
            }
            main.postDelayed(this, 2000);
        }
    };
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        home = new File(getFilesDir(), "pocol").getAbsolutePath();
        ScrollView scroll = new ScrollView(this); LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL); layout.setPadding(36, 60, 36, 36); scroll.addView(layout); setContentView(scroll);
        TextView title = new TextView(this); title.setText("PoCol Testnet"); title.setTextSize(28); layout.addView(title);
        TextView note = new TextView(this); note.setText("An independent node on this phone. Test coins only. Running uses battery, storage and network data. Android may stop the app. Keep the phone powered for a long test."); layout.addView(note);
        identity = new TextView(this); identity.setTextIsSelectable(true); layout.addView(identity);
        address = getPreferences(MODE_PRIVATE).getString("address", "");
        if (address.isEmpty()) try {
            JSONObject result = new JSONObject(Native.identity(home));
            address = result.optString("address");
            if (address.isEmpty()) identity.setText(result.toString());
            else getPreferences(MODE_PRIVATE).edit().putString("address", address).apply();
        } catch (JSONException e) { identity.setText("Cannot initialize identity"); }
        if (!address.isEmpty()) identity.setText("Public address\n" + address);
        button(layout, "Copy public address", () -> {
            getSystemService(android.content.ClipboardManager.class).setPrimaryClip(ClipData.newPlainText("PoCol public address", address));
        });
        button(layout, "Import network file", () -> startActivityForResult(new Intent(Intent.ACTION_OPEN_DOCUMENT).setType("application/json").addCategory(Intent.CATEGORY_OPENABLE), 1));
        button(layout, "Start PoCol", () -> {
            if (!new File(home, "network.pocol.json").isFile()) { status.setText("Import the shared network file first."); return; }
            if (Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS}, 2);
            try { startForegroundService(new Intent(this, NodeService.class)); attach(); status.setText("Starting local validation…"); }
            catch (RuntimeException e) { status.setText("Cannot start node: " + e.getMessage()); }
        });
        button(layout, "Stop PoCol", () -> { if (service != null) try { service.send(Message.obtain(null, 2)); status.setText("Stopping…"); } catch (RemoteException ignored) {} });
        status = new TextView(this); status.setTextIsSelectable(true); status.setText("Ready. Import the shared network file once, then start PoCol."); layout.addView(status);
    }
    private void button(LinearLayout layout, String text, Runnable action) { Button b = new Button(this); b.setText(text); b.setOnClickListener(v -> action.run()); layout.addView(b); }
    private void attach() { if (!bound) bound = bindService(new Intent(this, NodeService.class), connection, 0); }
    @Override protected void onStart() { super.onStart(); attach(); main.post(refresh); }
    @Override protected void onStop() { main.removeCallbacks(refresh); if (bound) { unbindService(connection); bound = false; service = null; } super.onStop(); }
    private void showStatus(String text) {
        try {
            JSONObject value = new JSONObject(text);
            if (value.has("head")) {
                status.setText("Height: " + value.getJSONObject("head").optLong("height") + "\nPeers: " + value.getJSONArray("peers").length() + "\n" + value.optString("production") + "\nHead: " + value.getJSONObject("head").optString("hash"));
            } else status.setText(value.toString(2));
        } catch (JSONException e) { status.setText("Status unavailable"); }
    }
    private static byte[] readLimited(InputStream in) throws IOException {
        ByteArrayOutputStream out = new ByteArrayOutputStream(); byte[] buffer = new byte[8192]; int n;
        while ((n = in.read(buffer)) != -1) { if (out.size() + n > 1048576) throw new IOException("File exceeds 1 MiB"); out.write(buffer, 0, n); }
        return out.toByteArray();
    }
    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != 1 || result != RESULT_OK || data == null || data.getData() == null) return;
        new Thread(() -> {
            File incoming = new File(getCacheDir(), "network-import.json");
            try (InputStream in = getContentResolver().openInputStream(data.getData())) {
                if (in == null) throw new IOException("Cannot read network file");
                byte[] bytes = readLimited(in);
                try (FileOutputStream out = new FileOutputStream(incoming)) { out.write(bytes); }
                String imported = Native.importNetwork(home, incoming.getAbsolutePath());
                main.post(() -> showStatus(imported));
            } catch (IOException e) { main.post(() -> status.setText("Import failed: " + e.getMessage())); }
            finally { incoming.delete(); }
        }, "network-import").start();
    }
}
