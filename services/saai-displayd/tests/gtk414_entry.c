/* APP-04 packed probe: focused GTK 4.14 Entry against saai-displayd.
 * Prints GTK_ENTRY_TEXT= on every change. Not a panther field.
 */
#include <gtk/gtk.h>
#include <stdio.h>
#include <stdlib.h>

static void on_text(GObject *obj, GParamSpec *pspec, gpointer data) {
    (void)pspec;
    (void)data;
    const char *text = gtk_editable_get_text(GTK_EDITABLE(obj));
    fprintf(stdout, "GTK_ENTRY_TEXT=%s\n", text ? text : "");
    fflush(stdout);
}

static gboolean quit_cb(gpointer data) {
    gtk_window_destroy(GTK_WINDOW(data));
    return G_SOURCE_REMOVE;
}

static void activate(GtkApplication *app, gpointer user_data) {
    (void)user_data;
    GtkWidget *win = gtk_application_window_new(app);
    GtkWidget *entry = gtk_entry_new();
    gtk_window_set_title(GTK_WINDOW(win), "saaios-gtk414-entry");
    gtk_widget_set_hexpand(entry, TRUE);
    gtk_widget_set_vexpand(entry, TRUE);
    gtk_window_set_default_size(GTK_WINDOW(win), 320, 200);
    gtk_window_set_child(GTK_WINDOW(win), entry);
    gtk_window_present(GTK_WINDOW(win));
    if (!getenv("GTK4_NO_GRAB")) {
        gtk_widget_grab_focus(entry);
    }
    if (getenv("GTK4_PROBE_HOLD")) {
        g_signal_connect(entry, "notify::text", G_CALLBACK(on_text), NULL);
    } else {
        g_timeout_add(2000, quit_cb, win);
    }
}

int main(int argc, char **argv) {
    GtkApplication *app =
        gtk_application_new("org.saaios.gtk414.entry", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
