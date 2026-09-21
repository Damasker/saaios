/* APP-04 packed probe: GtkDropDown popup without a tap.
 * gtk_widget_activate after map. GTK 4.10+ replacement for ComboBox.
 * Not a Y sweep. Not gtk4-demo. Not a panther field.
 */
#include <gtk/gtk.h>
#include <stdio.h>
#include <stdlib.h>

static gboolean quit_cb(gpointer data) {
    gtk_window_destroy(GTK_WINDOW(data));
    return G_SOURCE_REMOVE;
}

static gboolean popup_cb(gpointer data) {
    gtk_widget_activate(GTK_WIDGET(data));
    return G_SOURCE_REMOVE;
}

static void activate(GtkApplication *app, gpointer user_data) {
    (void)user_data;
    GtkWidget *win = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(win), "saaios-gtk414-dropdown");
    gtk_window_set_decorated(GTK_WINDOW(win), FALSE);
    gtk_window_set_default_size(GTK_WINDOW(win), 320, 200);
    const char *items[] = {"one", "two", "hi!", NULL};
    GtkWidget *drop = gtk_drop_down_new_from_strings(items);
    gtk_window_set_child(GTK_WINDOW(win), drop);
    gtk_window_present(GTK_WINDOW(win));
    g_timeout_add(200, popup_cb, drop);
    if (!getenv("GTK4_PROBE_HOLD")) {
        g_timeout_add(2000, quit_cb, win);
    }
}

int main(int argc, char **argv) {
    GtkApplication *app =
        gtk_application_new("org.saaios.gtk414.dropdown", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
