#include <Message.hxx>
#include <Message_Gravity.hxx>
#include <Message_Messenger.hxx>
#include <Message_Printer.hxx>
#include <Message_ProgressRange.hxx>
#include <bindings_common.hxx>

inline void Message_set_default_trace_level(Message_Gravity level) {
  const Message_SequenceOfPrinters &printers = Message::DefaultMessenger()->Printers();
  for (Message_SequenceOfPrinters::Iterator it(printers); it.More(); it.Next()) {
    it.Value()->SetTraceLevel(level);
  }
}

inline Standard_Integer Message_default_printer_count() { return Message::DefaultMessenger()->Printers().Length(); }
