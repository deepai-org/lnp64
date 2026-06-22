//===-- LNP64AsmParser.cpp - v2 assembly parser --------------------------===//
//
// Operand lexing + the TableGen-generated instruction matcher
// (LNP64GenAsmMatcher.inc). Mnemonic recognition and per-instruction operand
// shape/typing come from the .td AsmStrings via MatchInstructionImpl; this file
// only lexes operands (registers, immediates, the off(base)/(base) memory
// forms) into typed LNP64Operands and renders them.
//===----------------------------------------------------------------------===//

#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/ADT/StringSwitch.h"
#include "llvm/MC/MCContext.h"
#include "llvm/MC/MCExpr.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/MCParser/MCAsmLexer.h"
#include "llvm/MC/MCParser/MCAsmParser.h"
#include "llvm/MC/MCParser/MCParsedAsmOperand.h"
#include "llvm/MC/MCParser/MCTargetAsmParser.h"
#include "llvm/MC/MCStreamer.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/MathExtras.h"
#include "llvm/Support/SMLoc.h"

#include <memory>

using namespace llvm;

namespace {

class LNP64Operand final : public MCParsedAsmOperand {
public:
  enum KindTy { Token, Reg, Imm, Expr };

private:
  KindTy Kind;
  StringRef TokenValue;
  unsigned RegNo = 0;
  int64_t ImmValue = 0;
  const MCExpr *ExprValue = nullptr;
  SMLoc Start;
  SMLoc End;

public:
  LNP64Operand(KindTy Kind, SMLoc Start, SMLoc End)
      : Kind(Kind), Start(Start), End(End) {}

  static std::unique_ptr<LNP64Operand> createToken(StringRef Tok, SMLoc Loc) {
    auto Op = std::make_unique<LNP64Operand>(Token, Loc, Loc);
    Op->TokenValue = Tok;
    return Op;
  }
  static std::unique_ptr<LNP64Operand> createReg(unsigned RegNo, SMLoc S,
                                                 SMLoc E) {
    auto Op = std::make_unique<LNP64Operand>(Reg, S, E);
    Op->RegNo = RegNo;
    return Op;
  }
  static std::unique_ptr<LNP64Operand> createImm(int64_t V, SMLoc S, SMLoc E) {
    auto Op = std::make_unique<LNP64Operand>(Imm, S, E);
    Op->ImmValue = V;
    return Op;
  }
  static std::unique_ptr<LNP64Operand> createExpr(const MCExpr *V, SMLoc S,
                                                  SMLoc E) {
    auto Op = std::make_unique<LNP64Operand>(Expr, S, E);
    Op->ExprValue = V;
    return Op;
  }

  bool isToken() const override { return Kind == Token; }
  bool isReg() const override { return Kind == Reg; }
  bool isImm() const override { return Kind == Imm || Kind == Expr; }
  bool isMem() const override { return false; }
  // I-type / displacement immediates: a concrete signed-32 value.
  bool isSImm32() const { return Kind == Imm && isInt<32>(ImmValue); }

  StringRef getToken() const { return TokenValue; }
  unsigned getReg() const override { return RegNo; }
  int64_t getImm() const { return ImmValue; }
  const MCExpr *getExpr() const { return ExprValue; }

  SMLoc getStartLoc() const override { return Start; }
  SMLoc getEndLoc() const override { return End; }

  void addRegOperands(MCInst &Inst, unsigned N) const {
    assert(N == 1 && "register operand renders one MCOperand");
    (void)N;
    Inst.addOperand(MCOperand::createReg(RegNo));
  }
  void addImmOperands(MCInst &Inst, unsigned N) const {
    assert(N == 1 && "immediate operand renders one MCOperand");
    (void)N;
    if (Kind == Expr)
      Inst.addOperand(MCOperand::createExpr(ExprValue));
    else
      Inst.addOperand(MCOperand::createImm(ImmValue));
  }

  void print(raw_ostream &OS) const override {
    switch (Kind) {
    case Token: OS << TokenValue; break;
    case Reg: OS << "<reg " << RegNo << ">"; break;
    case Imm: OS << ImmValue; break;
    case Expr: ExprValue->print(OS, nullptr); break;
    }
  }
};

class LNP64AsmParser : public MCTargetAsmParser {
#define GET_ASSEMBLER_HEADER
#include "LNP64GenAsmMatcher.inc"

public:
  LNP64AsmParser(const MCSubtargetInfo &STI, MCAsmParser &Parser,
                 const MCInstrInfo &MII, const MCTargetOptions &Options)
      : MCTargetAsmParser(Options, STI, MII) {
    setAvailableFeatures(FeatureBitset());
  }

  bool ParseRegister(unsigned &RegNo, SMLoc &Start, SMLoc &End) override {
    if (getLexer().getKind() != AsmToken::Identifier)
      return true;
    unsigned Enc = 0;
    if (!parseRegisterName(getLexer().getTok().getIdentifier(), Enc))
      return true;
    Start = getLexer().getTok().getLoc();
    End = getLexer().getTok().getEndLoc();
    RegNo = Enc;
    getParser().Lex();
    return false;
  }

  OperandMatchResultTy tryParseRegister(unsigned &RegNo, SMLoc &Start,
                                        SMLoc &End) override {
    return ParseRegister(RegNo, Start, End) ? MatchOperand_NoMatch
                                            : MatchOperand_Success;
  }

  bool ParseDirective(AsmToken) override { return true; }

  bool ParseInstruction(ParseInstructionInfo &, StringRef Name, SMLoc NameLoc,
                        OperandVector &Operands) override {
    Operands.push_back(LNP64Operand::createToken(Name, NameLoc));
    if (getLexer().is(AsmToken::EndOfStatement)) {
      getParser().Lex();
      return false;
    }
    while (true) {
      if (parseOperand(Operands))
        return true;
      if (getLexer().is(AsmToken::EndOfStatement)) {
        getParser().Lex();
        return false;
      }
      if (!getLexer().is(AsmToken::Comma))
        return Error(getLexer().getTok().getLoc(),
                     "expected comma or end of statement");
      getParser().Lex();
    }
  }

  bool MatchAndEmitInstruction(SMLoc IDLoc, unsigned &, OperandVector &Operands,
                               MCStreamer &Out, uint64_t &ErrorInfo,
                               bool MatchingInlineAsm) override {
    MCInst Inst;
    switch (MatchInstructionImpl(Operands, Inst, ErrorInfo, MatchingInlineAsm)) {
    case Match_Success:
      Inst.setLoc(IDLoc);
      Out.emitInstruction(Inst, getSTI());
      return false;
    case Match_MissingFeature:
      return Error(IDLoc, "instruction requires a feature not currently enabled");
    case Match_MnemonicFail:
      return Error(IDLoc, "unrecognized LNP64 instruction mnemonic");
    case Match_InvalidOperand: {
      SMLoc ErrorLoc = IDLoc;
      if (ErrorInfo != ~0ULL) {
        if (ErrorInfo >= Operands.size())
          return Error(IDLoc, "too few operands for instruction");
        SMLoc Loc =
            static_cast<LNP64Operand &>(*Operands[ErrorInfo]).getStartLoc();
        if (Loc != SMLoc())
          ErrorLoc = Loc;
      }
      return Error(ErrorLoc, "invalid operand for instruction");
    }
    }
    llvm_unreachable("unknown match result");
  }

private:
  static bool parseRegisterName(StringRef Name, unsigned &RegNo) {
    unsigned PcrNo =
        StringSwitch<unsigned>(Name.upper())
            .Case("PID", LNP64::PID)
            .Case("PPID", LNP64::PPID)
            .Case("TID", LNP64::TID)
            .Case("TP", LNP64::TP)
            .Case("TLS_BASE", LNP64::TP)
            .Case("UID", LNP64::UID)
            .Case("POSIX_UID", LNP64::UID)
            .Case("GID", LNP64::GID)
            .Case("POSIX_GID", LNP64::GID)
            .Case("SIGMASK", LNP64::SIGMASK)
            .Case("SIGPENDING", LNP64::SIGPENDING)
            .Case("REALTIME_SEC", LNP64::REALTIME_SEC)
            .Case("REALTIME_NSEC", LNP64::REALTIME_NSEC)
            .Case("CRED_PROFILE", LNP64::CRED_PROFILE)
            .Case("CRED_HANDLE", LNP64::CRED_HANDLE)
            .Default(0);
    if (PcrNo) {
      RegNo = PcrNo;
      return true;
    }
    if (!Name.consume_front("r") && !Name.consume_front("R"))
      return false;
    unsigned Number = 0;
    if (Name.getAsInteger(10, Number) || Number > 31)
      return false;
    RegNo = LNP64::R0 + Number;
    return true;
  }

  bool parseRegOperand(OperandVector &Operands) {
    unsigned RegNo = 0;
    SMLoc S, E;
    if (ParseRegister(RegNo, S, E))
      return true;
    Operands.push_back(LNP64Operand::createReg(RegNo, S, E));
    return false;
  }

  // Push a "(reg)" group as the literal tokens the matcher's AsmString expects:
  // "(" <reg> ")".
  bool parseParenReg(OperandVector &Operands) {
    SMLoc LP = getLexer().getTok().getLoc();
    Operands.push_back(LNP64Operand::createToken("(", LP));
    getParser().Lex(); // eat '('
    if (parseRegOperand(Operands))
      return Error(getLexer().getTok().getLoc(), "expected base register");
    if (!getLexer().is(AsmToken::RParen))
      return Error(getLexer().getTok().getLoc(), "expected ')'");
    Operands.push_back(
        LNP64Operand::createToken(")", getLexer().getTok().getLoc()));
    getParser().Lex(); // eat ')'
    return false;
  }

  bool parseOperand(OperandVector &Operands) {
    if (getLexer().is(AsmToken::LParen))
      return parseParenReg(Operands);

    if (getLexer().is(AsmToken::Identifier)) {
      if (!parseRegOperand(Operands))
        return false;
      // Not a register: a symbol/expression operand.
      SMLoc S = getLexer().getTok().getLoc();
      const MCExpr *Val = nullptr;
      if (getParser().parseExpression(Val))
        return true;
      Operands.push_back(
          LNP64Operand::createExpr(Val, S, getLexer().getTok().getLoc()));
      return false;
    }

    if (getLexer().is(AsmToken::Integer) || getLexer().is(AsmToken::Minus)) {
      SMLoc S = getLexer().getTok().getLoc();
      bool Neg = false;
      if (getLexer().is(AsmToken::Minus)) {
        Neg = true;
        getParser().Lex();
      }
      if (!getLexer().is(AsmToken::Integer))
        return Error(getLexer().getTok().getLoc(), "expected integer immediate");
      int64_t V = getLexer().getTok().getIntVal();
      if (Neg)
        V = -V;
      SMLoc E = getLexer().getTok().getEndLoc();
      getParser().Lex();
      Operands.push_back(LNP64Operand::createImm(V, S, E));
      // displacement form: imm "(" base ")"
      if (getLexer().is(AsmToken::LParen))
        return parseParenReg(Operands);
      return false;
    }

    return Error(getLexer().getTok().getLoc(),
                 "expected register, immediate, or memory operand");
  }
};

} // end anonymous namespace

#define GET_REGISTER_MATCHER
#define GET_MATCHER_IMPLEMENTATION
#include "LNP64GenAsmMatcher.inc"

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64AsmParser() {
  RegisterMCAsmParser<LNP64AsmParser> X(getTheLNP64Target());
}
