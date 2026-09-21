// APP-04 host probe: focused QLineEdit against saai-displayd.
// Prints QT_LINEEDIT_TEXT= on every change. Not a panther field.
#include <QApplication>
#include <QLineEdit>
#include <cstdio>

int main(int argc, char **argv) {
    qputenv("QT_QPA_PLATFORM", "wayland");
    QApplication app(argc, argv);
    QLineEdit edit;
    edit.setPlaceholderText(QStringLiteral("local field"));
    edit.resize(320, 40);
    QObject::connect(&edit, &QLineEdit::textChanged, [](const QString &text) {
        std::fprintf(stdout, "QT_LINEEDIT_TEXT=%s\n", text.toUtf8().constData());
        std::fflush(stdout);
    });
    edit.show();
    edit.setFocus(Qt::OtherFocusReason);
    return app.exec();
}
