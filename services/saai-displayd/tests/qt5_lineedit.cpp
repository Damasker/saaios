// APP-04 host probe: focused QLineEdit against saai-displayd.
// Prints QT_LINEEDIT_TEXT= on every change. Not a panther field.
#include <QApplication>
#include <QCompleter>
#include <QLineEdit>
#include <QMenu>
#include <QStringList>
#include <QTimer>
#include <QVBoxLayout>
#include <QWidget>
#include <cstdio>

int main(int argc, char **argv) {
    qputenv("QT_QPA_PLATFORM", "wayland");
    QApplication app(argc, argv);

    QLineEdit *edit = nullptr;
    QWidget window;
    const int steal_ms = qEnvironmentVariableIntValue("QT_LINEEDIT_STEAL_MS");
    if (qEnvironmentVariableIsSet("QT_LINEEDIT_COMPETE")) {
        // ADR-360: LocationBar/Filter class. A non-IM pane holds
        // focus (WebView/FolderView). The field is the top 40 px.
        // ADR-361: QT_LINEEDIT_STEAL_MS > 0 returns focus to the pane
        // after the field is focused (WebEngine steal-back).
        window.resize(320, 200);
        window.setWindowFlags(Qt::FramelessWindowHint);
        auto *layout = new QVBoxLayout(&window);
        layout->setContentsMargins(0, 0, 0, 0);
        layout->setSpacing(0);
        edit = new QLineEdit;
        edit->setPlaceholderText(QStringLiteral("local field"));
        edit->setFixedHeight(40);
        auto *pane = new QWidget;
        pane->setFocusPolicy(Qt::StrongFocus);
        pane->setMinimumHeight(160);
        layout->addWidget(edit);
        layout->addWidget(pane, 1);
        if (steal_ms > 0) {
            QObject::connect(
                &app,
                &QApplication::focusChanged,
                edit,
                [edit, pane, steal_ms](QWidget *, QWidget *now) {
                    if (now == edit) {
                        QTimer::singleShot(steal_ms, pane, [pane]() {
                            pane->setFocus(Qt::OtherFocusReason);
                        });
                    }
                });
        }
        pane->setFocus(Qt::OtherFocusReason);
        window.show();
    } else {
        edit = new QLineEdit;
        edit->setPlaceholderText(QStringLiteral("local field"));
        edit->resize(320, 40);
        edit->show();
        // ADR-359: a lone QLineEdit auto-focuses on Activated.
        if (!qEnvironmentVariableIsSet("QT_LINEEDIT_NO_SETFOCUS")) {
            edit->setFocus(Qt::OtherFocusReason);
        }
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_COMPLETER")) {
        // ADR-394: PathEdit/LocationBar class. QCompleter popup is an
        // xdg_popup; empty new_popup left it unmapped.
        auto *completer = new QCompleter(
            QStringList() << QStringLiteral("hi!") << QStringLiteral("hello"),
            edit);
        edit->setCompleter(completer);
        QTimer::singleShot(80, completer, [completer]() { completer->complete(); });
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_SELECTALL")) {
        // ADR-408: PathEdit click class. selectAll with path
        // surrounding. Lone QLineEdit keeps v2; PathEdit disable
        // is not this.
        edit->setText(QStringLiteral("/home/mike/worktrees/saaios-som"));
        auto select = [edit]() { edit->selectAll(); };
        QTimer::singleShot(0, edit, select);
        QObject::connect(
            &app,
            &QApplication::focusChanged,
            edit,
            [edit](QWidget *, QWidget *now) {
                if (now == edit) {
                    QTimer::singleShot(0, edit, [edit]() { edit->selectAll(); });
                }
            });
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_COMPLETER_RELOAD")) {
        // ADR-409: PathEdit focusInEvent reloads QCompleter and does
        // not complete(). Lone QLineEdit keeps v2; PathEdit disable
        // is not this.
        auto reload = [edit]() {
            auto *old = edit->completer();
            auto *c = new QCompleter(
                QStringList() << QStringLiteral("hi!") << QStringLiteral("hello"),
                edit);
            edit->setCompleter(c);
            if (old) {
                old->deleteLater();
            }
        };
        QTimer::singleShot(0, edit, reload);
        QObject::connect(
            &app,
            &QApplication::focusChanged,
            edit,
            [edit, reload](QWidget *, QWidget *now) {
                if (now == edit) {
                    QTimer::singleShot(0, edit, reload);
                } else if (edit->completer()) {
                    auto *old = edit->completer();
                    edit->setCompleter(nullptr);
                    old->deleteLater();
                }
            });
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_MENU")) {
        // ADR-403: QMenu::popup. Qt Wayland menus are a second
        // xdg_toplevel, same class as QCompleter — not xdg_popup.
        auto *menu = new QMenu(edit);
        menu->addAction(QStringLiteral("hi"));
        QTimer::singleShot(200, menu, [menu, edit]() {
            menu->popup(edit->mapToGlobal(QPoint(8, 8)));
        });
    }

    QObject::connect(edit, &QLineEdit::textChanged, [](const QString &text) {
        std::fprintf(stdout, "QT_LINEEDIT_TEXT=%s\n", text.toUtf8().constData());
        std::fflush(stdout);
    });
    return app.exec();
}
