package dev.playhorizon.unofficial.android;

import android.app.NativeActivity;
import android.content.ContentResolver;
import android.content.ContentValues;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.MediaStore;
import android.view.InputDevice;
import android.view.View;
import android.view.WindowManager;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.OutputStream;

/**
 * The game's activity: NativeActivity, which loads libhorizon.so and hands it the window and
 * the input, with the whole screen for the game: nothing of the system over it, and the
 * camera cutout's sides drawn on too.
 */
public class HorizonActivity extends NativeActivity {
    /** Whether the game wants the mouse captured (it's playing, with no screen open). */
    private volatile boolean wantsCapture;

    /** The name of the last run's log in Downloads (exportLastLog). */
    private static final String LAST_LOG = "Ferrite-last-log.txt";

    @Override
    protected void onCreate(Bundle saved) {
        // Before the game starts: it begins a new log over the last one.
        exportLastLog();
        super.onCreate(saved);
        if (Build.VERSION.SDK_INT >= 28) {
            WindowManager.LayoutParams attributes = getWindow().getAttributes();
            attributes.layoutInDisplayCutoutMode =
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            getWindow().setAttributes(attributes);
        }
        hideSystemBars();
    }

    @Override
    public void onWindowFocusChanged(boolean focused) {
        super.onWindowFocusChanged(focused);
        if (focused) {
            hideSystemBars();
            // Android ends pointer capture when the window loses focus.
            if (wantsCapture) {
                setPointerCapture(true);
            }
        }
    }

    /**
     * Captures the mouse for looking around (the pointer hidden, its moves relative), or lets it
     * go for menus. Called by the game (horizon_threads::android_sys::pointer_capture) from its
     * own thread; a view's capture changes on the UI thread. Without a mouse it does nothing:
     * fingers are never captured.
     */
    public void setPointerCapture(final boolean on) {
        wantsCapture = on;
        if (Build.VERSION.SDK_INT < 26) {
            return;
        }
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                View view = getWindow().getDecorView();
                if (wantsCapture && hasMouse()) {
                    view.requestPointerCapture();
                } else {
                    view.releasePointerCapture();
                }
            }
        });
    }

    /**
     * Copies the last run's log (home/logs/latest.log in the game folder) to Downloads as
     * Ferrite-last-log.txt, replacing the copy before. Android keeps file managers out of the
     * game folder (Android/data), so this is how a player finds the log of a run that crashed:
     * open the app again, and the log is in Downloads. Nothing happens without a log.
     */
    private void exportLastLog() {
        if (Build.VERSION.SDK_INT < 29) {
            return;
        }
        try {
            File dir = getExternalFilesDir(null);
            File log = dir == null ? null : new File(dir, "home/logs/latest.log");
            if (log == null || !log.isFile()) {
                return;
            }
            ContentResolver resolver = getContentResolver();
            // Only this app's own earlier copy: an app can remove only the files it made.
            resolver.delete(MediaStore.Downloads.EXTERNAL_CONTENT_URI,
                    MediaStore.MediaColumns.DISPLAY_NAME + "=?", new String[] {LAST_LOG});
            ContentValues values = new ContentValues();
            values.put(MediaStore.MediaColumns.DISPLAY_NAME, LAST_LOG);
            values.put(MediaStore.MediaColumns.MIME_TYPE, "text/plain");
            Uri copy = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values);
            if (copy == null) {
                return;
            }
            try (InputStream in = new FileInputStream(log);
                 OutputStream out = resolver.openOutputStream(copy)) {
                if (out == null) {
                    return;
                }
                byte[] buffer = new byte[64 * 1024];
                int read;
                while ((read = in.read(buffer)) > 0) {
                    out.write(buffer, 0, read);
                }
            }
        } catch (Exception e) {
            // The game starts either way.
        }
    }

    /** Whether a mouse or a touchpad is connected. */
    private static boolean hasMouse() {
        for (int id : InputDevice.getDeviceIds()) {
            InputDevice device = InputDevice.getDevice(id);
            if (device != null && device.supportsSource(InputDevice.SOURCE_MOUSE)) {
                return true;
            }
        }
        return false;
    }

    /** Immersive mode: a swipe from an edge shows the bars for a moment, over the game. */
    @SuppressWarnings("deprecation")
    private void hideSystemBars() {
        getWindow().getDecorView().setSystemUiVisibility(
                View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                        | View.SYSTEM_UI_FLAG_FULLSCREEN
                        | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                        | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                        | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                        | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION);
    }
}
