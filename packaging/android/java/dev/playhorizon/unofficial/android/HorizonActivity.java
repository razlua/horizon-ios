package dev.playhorizon.unofficial.android;

import android.app.NativeActivity;
import android.os.Build;
import android.os.Bundle;
import android.view.InputDevice;
import android.view.View;
import android.view.WindowManager;

/**
 * The game's activity: NativeActivity, which loads libhorizon.so and hands it the window and
 * the input, with the whole screen for the game: nothing of the system over it, and the
 * camera cutout's sides drawn on too.
 */
public class HorizonActivity extends NativeActivity {
    /** Whether the game wants the mouse captured (it's playing, with no screen open). */
    private volatile boolean wantsCapture;

    @Override
    protected void onCreate(Bundle saved) {
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
