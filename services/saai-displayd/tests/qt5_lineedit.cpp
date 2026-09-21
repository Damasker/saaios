// APP-04 host probe: focused QLineEdit against saai-displayd.
// Prints QT_LINEEDIT_TEXT= on every change. Not a panther field.
#include <QApplication>
#include <QCompleter>
#include <QCoreApplication>
#include <QInputMethodEvent>
#include <QLineEdit>
#include <QMenu>
#include <QStringList>
#include <QStringListModel>
#include <QThread>
#include <QTimer>
#include <QUrl>
#include <QVBoxLayout>
#include <QWebEngineView>
#include <QWidget>
#include <QWindow>
#include <cstdio>

int main(int argc, char **argv) {
    qputenv("QT_QPA_PLATFORM", "wayland");
    QApplication app(argc, argv);

    QLineEdit *edit = nullptr;
    QWidget window;
    const int steal_ms = qEnvironmentVariableIntValue("QT_LINEEDIT_STEAL_MS");
    if (qEnvironmentVariableIsSet("QT_LINEEDIT_WEBENGINE")) {
        // ADR-418: Falkon LocationBar above QWebEngineView.
        // Sibling about:blank does not steal IM; Falkon disable
        // is not this.
        window.resize(640, 400);
        window.setWindowFlags(Qt::FramelessWindowHint);
        auto *layout = new QVBoxLayout(&window);
        layout->setContentsMargins(0, 0, 0, 0);
        layout->setSpacing(0);
        edit = new QLineEdit;
        edit->setText(QStringLiteral("https://example.com"));
        edit->setFixedHeight(40);
        auto *view = new QWebEngineView;
        view->load(QUrl(QStringLiteral("about:blank")));
        layout->addWidget(edit);
        layout->addWidget(view, 1);
        window.show();
        edit->setFocus(Qt::OtherFocusReason);
    } else if (qEnvironmentVariableIsSet("QT_LINEEDIT_COMPETE")) {
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

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_COMPLETER_ASYNC")) {
        // ADR-411: PathEdit onJobFinished updates the model without
        // complete() on focusIn. Lone QLineEdit keeps v2; PathEdit
        // disable is not this.
        edit->setText(QStringLiteral("/tmp/"));
        auto *model = new QStringListModel(edit);
        auto *c = new QCompleter(edit);
        c->setModel(model);
        edit->setCompleter(c);
        QTimer::singleShot(80, model, [model]() {
            model->setStringList(QStringList()
                                 << QStringLiteral("/tmp/a/")
                                 << QStringLiteral("/tmp/b/"));
        });
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_COMPLETER_BLOCK")) {
        // ADR-412: PathEdit GIO BlockingQueuedConnection into
        // onJobFinished. Lone QLineEdit keeps v2; PathEdit disable
        // is not this.
        edit->setText(QStringLiteral("/tmp/"));
        auto *model = new QStringListModel(edit);
        auto *c = new QCompleter(edit);
        c->setModel(model);
        edit->setCompleter(c);
        auto *thread = new QThread(edit);
        auto *worker = new QObject();
        worker->moveToThread(thread);
        QObject::connect(thread, &QThread::started, worker, [model, worker]() {
            QMetaObject::invokeMethod(
                model,
                [model]() {
                    model->setStringList(QStringList()
                                         << QStringLiteral("/tmp/a/")
                                         << QStringLiteral("/tmp/b/"));
                },
                Qt::BlockingQueuedConnection);
            QThread::currentThread()->quit();
            worker->deleteLater();
        });
        QObject::connect(thread, &QThread::finished, thread, &QObject::deleteLater);
        QTimer::singleShot(80, thread, [thread]() { thread->start(QThread::LowPriority); });
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_URL_HINTS")) {
        // ADR-413: Falkon LocationBar ImhNoAutoUppercase |
        // ImhUrlCharactersOnly. Lone QLineEdit keeps v2; Falkon
        // disable is not this.
        edit->setText(QStringLiteral("https://example.com"));
        edit->setInputMethodHints(Qt::ImhNoAutoUppercase | Qt::ImhUrlCharactersOnly);
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_INLINE")) {
        // ADR-414: Falkon LocationBar domain QCompleter is
        // InlineCompletion. Lone QLineEdit keeps v2; Falkon disable
        // is not this.
        edit->setText(QStringLiteral("https://example.com"));
        auto *model = new QStringListModel(edit);
        model->setStringList(QStringList() << QStringLiteral("https://example.com/"));
        auto *c = new QCompleter(edit);
        c->setCompletionMode(QCompleter::InlineCompletion);
        c->setModel(model);
        edit->setCompleter(c);
        QTimer::singleShot(80, c, [c]() { c->complete(); });
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_TOOLTIP_POPUP")) {
        // ADR-415: Falkon LocationCompleterView is Qt::ToolTip +
        // setFocusProxy. Maps xdg_popup and keeps v2; Falkon
        // disable is not this.
        edit->setText(QStringLiteral("https://example.com"));
        QTimer::singleShot(80, edit, [edit]() {
            auto *popup = new QWidget(nullptr);
            popup->setAttribute(Qt::WA_ShowWithoutActivating);
            popup->setAttribute(Qt::WA_DeleteOnClose);
            popup->setWindowFlags(Qt::ToolTip | Qt::FramelessWindowHint
                                  | Qt::BypassWindowManagerHint);
            popup->resize(240, 80);
            popup->setFocusProxy(edit);
            popup->createWinId();
            if (popup->windowHandle() && edit->window()->windowHandle()) {
                popup->windowHandle()->setTransientParent(
                    edit->window()->windowHandle());
            }
            popup->move(edit->mapToGlobal(QPoint(0, edit->height())));
            popup->show();
        });
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_IM_FORMAT")) {
        // ADR-416: Falkon LineEdit::clearTextFormat empty
        // QInputMethodEvent. Lone QLineEdit keeps v2; Falkon
        // disable is not this.
        edit->setText(QStringLiteral("https://example.com"));
        QTimer::singleShot(80, edit, [edit]() {
            QList<QInputMethodEvent::Attribute> attrs;
            QInputMethodEvent ev(QString(), attrs);
            QCoreApplication::sendEvent(edit, &ev);
        });
    }

    if (qEnvironmentVariableIsSet("QT_LINEEDIT_SIDE")) {
        // ADR-417: Falkon LineEdit SideWidget ClickFocus +
        // setGoIconVisible margins. Lone QLineEdit keeps v2;
        // Falkon disable is not this.
        edit->setText(QStringLiteral("https://example.com"));
        auto *left = new QWidget(edit);
        left->setFocusPolicy(Qt::ClickFocus);
        left->setFixedSize(24, 24);
        left->move(2, 8);
        left->show();
        auto *go = new QWidget(edit);
        go->setFocusPolicy(Qt::ClickFocus);
        go->setFixedSize(24, 24);
        go->move(280, 8);
        go->hide();
        edit->setTextMargins(28, 0, 4, 0);
        QTimer::singleShot(80, edit, [edit, left, go]() {
            left->hide();
            go->show();
            edit->setTextMargins(28, 0, 28, 0);
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
