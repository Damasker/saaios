/* APP-04 packed probe: GtkMenuButton popover without a tap.
 * gtk_menu_button_popup after map. Not a Y sweep. Not gtk4-demo.
 * GTK4_POPOVER_ENTRY=1: Entry child + grab_focus after popup.
 * Not a panther field.
 */
#include <gtk/gtk.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    GtkWidget *btn;
    GtkWidget *entry;
} Pop;

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

static gboolean popup_cb(gpointer data) {
    Pop *p = data;
    gtk_menu_button_popup(GTK_MENU_BUTTON(p->btn));
    if (p->entry) {
        gtk_widget_grab_focus(p->entry);
    }
    g_free(p);
    return G_SOURCE_REMOVE;
}

static void activate(GtkApplication *app, gpointer user_data) {
    (void)user_data;
    GtkWidget *win = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(win), "saaios-gtk414-popover");
    gtk_window_set_decorated(GTK_WINDOW(win), FALSE);
    gtk_window_set_default_size(GTK_WINDOW(win), 320, 200);
    GtkWidget *btn = gtk_menu_button_new();
    gtk_menu_button_set_label(GTK_MENU_BUTTON(btn), "pop");
    GtkWidget *pop = gtk_popover_new();
    GtkWidget *entry = NULL;
    if (getenv("GTK4_POPOVER_ENTRY")) {
        entry = gtk_entry_new();
        gtk_popover_set_child(GTK_POPOVER(pop), entry);
        g_signal_connect(entry, "notify::text", G_CALLBACK(on_text), NULL);
    } else {
        gtk_popover_set_child(GTK_POPOVER(pop), gtk_label_new("hi"));
    }
    gtk_menu_button_set_popover(GTK_MENU_BUTTON(btn), pop);
    gtk_window_set_child(GTK_WINDOW(win), btn);
    gtk_window_present(GTK_WINDOW(win));
    Pop *payload = g_new0(Pop, 1);
    payload->btn = btn;
    payload->entry = entry;
    g_timeout_add(200, popup_cb, payload);
    if (!getenv("GTK4_PROBE_HOLD")) {
        g_timeout_add(2000, quit_cb, win);
    }
}

int main(int argc, char **argv) {
    GtkApplication *app =
        gtk_application_new("org.saaios.gtk414.popover", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
