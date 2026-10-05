/* Alpine GTK 4.14.4 Entry against host saai-displayd (ADR-322). */
#include <gtk/gtk.h>

static gboolean quit_cb(gpointer data) {
    gtk_window_destroy(GTK_WINDOW(data));
    return G_SOURCE_REMOVE;
}

static void activate(GtkApplication *app, gpointer user_data) {
    (void)user_data;
    GtkWidget *win = gtk_application_window_new(app);
    GtkWidget *box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 8);
    GtkWidget *entry = gtk_entry_new();
    gtk_window_set_title(GTK_WINDOW(win), "saaios-gtk414-entry");
    gtk_window_set_default_size(GTK_WINDOW(win), 320, 240);
    gtk_box_append(GTK_BOX(box), gtk_label_new("hello"));
    gtk_box_append(GTK_BOX(box), entry);
    gtk_window_set_child(GTK_WINDOW(win), box);
    gtk_window_present(GTK_WINDOW(win));
    gtk_widget_grab_focus(entry);
    g_timeout_add(2000, quit_cb, win);
}

int main(int argc, char **argv) {
    GtkApplication *app =
        gtk_application_new("org.saaios.gtk414.entry", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
