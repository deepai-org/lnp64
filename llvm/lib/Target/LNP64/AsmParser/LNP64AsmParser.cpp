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
#include "llvm/Support/SMLoc.h"

#include <memory>

using namespace llvm;

namespace {

class LNP64Operand final : public MCParsedAsmOperand {
public:
  enum KindTy { Token, Reg, Imm, Expr, Mem };

private:
  KindTy Kind;
  StringRef TokenValue;
  unsigned RegNo = 0;
  int64_t ImmValue = 0;
  const MCExpr *ExprValue = nullptr;
  unsigned BaseRegNo = 0;
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

  static std::unique_ptr<LNP64Operand> createReg(unsigned RegNo, SMLoc Start,
                                                SMLoc End) {
    auto Op = std::make_unique<LNP64Operand>(Reg, Start, End);
    Op->RegNo = RegNo;
    return Op;
  }

  static std::unique_ptr<LNP64Operand> createImm(int64_t Value, SMLoc Start,
                                                SMLoc End) {
    auto Op = std::make_unique<LNP64Operand>(Imm, Start, End);
    Op->ImmValue = Value;
    return Op;
  }

  static std::unique_ptr<LNP64Operand> createExpr(const MCExpr *ExprValue,
                                                 SMLoc Start, SMLoc End) {
    auto Op = std::make_unique<LNP64Operand>(Expr, Start, End);
    Op->ExprValue = ExprValue;
    return Op;
  }

  static std::unique_ptr<LNP64Operand> createMem(int64_t Offset,
                                                unsigned BaseRegNo,
                                                SMLoc Start, SMLoc End) {
    auto Op = std::make_unique<LNP64Operand>(Mem, Start, End);
    Op->ImmValue = Offset;
    Op->BaseRegNo = BaseRegNo;
    return Op;
  }

  bool isToken() const override { return Kind == Token; }
  bool isReg() const override { return Kind == Reg; }
  bool isImm() const override { return Kind == Imm || Kind == Expr; }
  bool isMem() const override { return Kind == Mem; }

  StringRef getToken() const { return TokenValue; }
  unsigned getReg() const override { return RegNo; }
  int64_t getImm() const { return ImmValue; }
  const MCExpr *getExpr() const { return ExprValue; }
  bool isImmValue() const { return Kind == Imm; }
  bool isExprValue() const { return Kind == Expr; }
  unsigned getBaseReg() const { return BaseRegNo; }

  SMLoc getStartLoc() const override { return Start; }
  SMLoc getEndLoc() const override { return End; }

  void print(raw_ostream &OS) const override {
    switch (Kind) {
    case Token:
      OS << TokenValue;
      break;
    case Reg:
      OS << "r" << (RegNo - LNP64::R0);
      break;
    case Imm:
      OS << ImmValue;
      break;
    case Expr:
      ExprValue->print(OS, nullptr);
      break;
    case Mem:
      OS << ImmValue << "(r" << (BaseRegNo - LNP64::R0) << ")";
      break;
    }
  }
};

class LNP64AsmParser : public MCTargetAsmParser {
public:
  LNP64AsmParser(const MCSubtargetInfo &STI, MCAsmParser &Parser,
                 const MCInstrInfo &MII, const MCTargetOptions &Options)
      : MCTargetAsmParser(Options, STI, MII) {
    setAvailableFeatures(FeatureBitset());
  }

  bool ParseRegister(unsigned &RegNo, SMLoc &Start, SMLoc &End) override {
    if (getLexer().getKind() != AsmToken::Identifier)
      return true;

    StringRef Name = getLexer().getTok().getIdentifier();
    unsigned Enc = 0;
    if (!parseRegisterName(Name, Enc))
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

  void convertToMapAndConstraints(unsigned, const OperandVector &) override {}

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
                               MCStreamer &Out, uint64_t &, bool) override {
    if (Operands.empty() ||
        !static_cast<LNP64Operand *>(Operands[0].get())->isToken())
      return Error(IDLoc, "expected LNP64 mnemonic");

    StringRef Mnemonic =
        static_cast<LNP64Operand *>(Operands[0].get())->getToken();
    MCInst Inst;
    if (!buildInstruction(Mnemonic, Operands, Inst))
      return Error(IDLoc, "invalid LNP64 operands for instruction");

    Out.emitInstruction(Inst, getSTI());
    return false;
  }

private:
  static bool parseRegisterName(StringRef Name, unsigned &RegNo) {
    if (!Name.consume_front("r") && !Name.consume_front("R"))
      return false;
    unsigned Number = 0;
    if (Name.getAsInteger(10, Number) || Number > 31)
      return false;
    RegNo = LNP64::R0 + Number;
    return true;
  }

  bool parseOperand(OperandVector &Operands) {
    if (getLexer().is(AsmToken::Identifier)) {
      unsigned RegNo = 0;
      SMLoc Start;
      SMLoc End;
      if (!ParseRegister(RegNo, Start, End)) {
        Operands.push_back(LNP64Operand::createReg(RegNo, Start, End));
        return false;
      }
      return parseExpressionOperand(Operands);
    }

    if (getLexer().is(AsmToken::Integer) || getLexer().is(AsmToken::Minus))
      return parseImmediateOrMemory(Operands);

    return Error(getLexer().getTok().getLoc(),
                 "expected register, immediate, or memory operand");
  }

  bool parseExpressionOperand(OperandVector &Operands) {
    SMLoc Start = getLexer().getTok().getLoc();
    const MCExpr *ExprValue = nullptr;
    if (getParser().parseExpression(ExprValue))
      return true;
    Operands.push_back(
        LNP64Operand::createExpr(ExprValue, Start, getLexer().getTok().getLoc()));
    return false;
  }

  bool parseImmediateOrMemory(OperandVector &Operands) {
    SMLoc Start = getLexer().getTok().getLoc();
    bool Negative = false;
    if (getLexer().is(AsmToken::Minus)) {
      Negative = true;
      getParser().Lex();
    }

    if (!getLexer().is(AsmToken::Integer))
      return Error(getLexer().getTok().getLoc(), "expected integer immediate");

    int64_t Value = getLexer().getTok().getIntVal();
    if (Negative)
      Value = -Value;
    SMLoc End = getLexer().getTok().getEndLoc();
    getParser().Lex();

    if (!getLexer().is(AsmToken::LParen)) {
      Operands.push_back(LNP64Operand::createImm(Value, Start, End));
      return false;
    }

    getParser().Lex();
    unsigned BaseRegNo = 0;
    SMLoc BaseStart;
    SMLoc BaseEnd;
    if (ParseRegister(BaseRegNo, BaseStart, BaseEnd))
      return Error(getLexer().getTok().getLoc(), "expected base register");
    if (!getLexer().is(AsmToken::RParen))
      return Error(getLexer().getTok().getLoc(), "expected ')'");
    End = getLexer().getTok().getEndLoc();
    getParser().Lex();

    Operands.push_back(LNP64Operand::createMem(Value, BaseRegNo, Start, End));
    return false;
  }

  static const LNP64Operand *getOp(const OperandVector &Operands, unsigned I) {
    if (I >= Operands.size())
      return nullptr;
    return static_cast<LNP64Operand *>(Operands[I].get());
  }

  static bool buildInstruction(StringRef Mnemonic, const OperandVector &Operands,
                               MCInst &Inst) {
    unsigned Opcode =
        StringSwitch<unsigned>(Mnemonic)
            .Case("nop", LNP64::NOP)
            .Case("li", LNP64::LI)
            .Case("li32", LNP64::LI32)
            .Case("la", LNP64::LA)
            .Case("mov", LNP64::MOV)
            .Case("add", LNP64::ADD)
            .Case("sub", LNP64::SUB)
            .Case("mul", LNP64::MUL)
            .Case("div", LNP64::DIV)
            .Case("and", LNP64::AND)
            .Case("or", LNP64::OR)
            .Case("xor", LNP64::XOR)
            .Case("not", LNP64::NOT)
            .Case("lsl", LNP64::LSL)
            .Case("lsr", LNP64::LSR)
            .Case("asr", LNP64::ASR)
            .Case("cmp", LNP64::CMP)
            .Case("cmpu", LNP64::CMPU)
            .Case("cset.eq", LNP64::CSET_EQ)
            .Case("cset.ne", LNP64::CSET_NE)
            .Case("cset.lt", LNP64::CSET_LT)
            .Case("cset.gt", LNP64::CSET_GT)
            .Case("cset.le", LNP64::CSET_LE)
            .Case("cset.ge", LNP64::CSET_GE)
            .Case("cset.ult", LNP64::CSET_ULT)
            .Case("cset.ugt", LNP64::CSET_UGT)
            .Case("cset.ule", LNP64::CSET_ULE)
            .Case("cset.uge", LNP64::CSET_UGE)
            .Case("jmp", LNP64::JMP)
            .Case("beq", LNP64::BEQ)
            .Case("bne", LNP64::BNE)
            .Case("blt", LNP64::BLT)
            .Case("bgt", LNP64::BGT)
            .Case("ble", LNP64::BLE)
            .Case("bge", LNP64::BGE)
            .Case("call", LNP64::CALL)
            .Case("call_reg", LNP64::CALL_REG)
            .Case("ret", LNP64::RET)
            .Case("errno_get", LNP64::ERRNO_GET)
            .Case("errno_set", LNP64::ERRNO_SET)
            .Case("exit", LNP64::EXIT)
            .Case("pull", LNP64::PULL)
            .Case("push", LNP64::PUSH)
            .Case("ld", LNP64::LD)
            .Case("ld.w", LNP64::LD_W)
            .Case("ld.h", LNP64::LD_H)
            .Case("ld.b", LNP64::LD_B)
            .Case("st", LNP64::ST)
            .Case("st.w", LNP64::ST_W)
            .Case("st.h", LNP64::ST_H)
            .Case("st.b", LNP64::ST_B)
            .Default(0);

    if (Opcode == 0)
      return false;

    Inst.setOpcode(Opcode);
    if (Opcode == LNP64::NOP || Opcode == LNP64::RET)
      return Operands.size() == 1;

    if (Opcode == LNP64::LI)
      return addRegImm(Inst, Operands);
    if (Opcode == LNP64::LA || Opcode == LNP64::LI32)
      return addRegAddress(Inst, Operands);
    if (Opcode == LNP64::MOV || Opcode == LNP64::NOT)
      return addRegReg(Inst, Operands);
    if (Opcode == LNP64::ADD || Opcode == LNP64::SUB ||
        Opcode == LNP64::MUL || Opcode == LNP64::DIV ||
        Opcode == LNP64::AND || Opcode == LNP64::OR ||
        Opcode == LNP64::XOR || Opcode == LNP64::LSL ||
        Opcode == LNP64::LSR || Opcode == LNP64::ASR)
      return addRegRegReg(Inst, Operands);
    if (Opcode == LNP64::CMP || Opcode == LNP64::CMPU)
      return addRegReg(Inst, Operands);
    if (Opcode == LNP64::JMP || Opcode == LNP64::BEQ ||
        Opcode == LNP64::BNE || Opcode == LNP64::BLT ||
        Opcode == LNP64::BGT || Opcode == LNP64::BLE ||
        Opcode == LNP64::BGE || Opcode == LNP64::CALL)
      return addBranchTarget(Inst, Operands);
    if (Opcode == LNP64::CALL_REG || Opcode == LNP64::CSET_EQ ||
        Opcode == LNP64::CSET_NE || Opcode == LNP64::CSET_LT ||
        Opcode == LNP64::CSET_GT || Opcode == LNP64::CSET_LE ||
        Opcode == LNP64::CSET_GE || Opcode == LNP64::CSET_ULT ||
        Opcode == LNP64::CSET_UGT || Opcode == LNP64::CSET_ULE ||
        Opcode == LNP64::CSET_UGE)
      return addReg(Inst, Operands);
    if (Opcode == LNP64::ERRNO_GET || Opcode == LNP64::ERRNO_SET ||
        Opcode == LNP64::EXIT)
      return addReg(Inst, Operands);
    if (Opcode == LNP64::PULL || Opcode == LNP64::PUSH)
      return addRegRegRegReg(Inst, Operands);
    if (Opcode == LNP64::LD || Opcode == LNP64::LD_W ||
        Opcode == LNP64::LD_H || Opcode == LNP64::LD_B)
      return addLoad(Inst, Operands);
    if (Opcode == LNP64::ST || Opcode == LNP64::ST_W ||
        Opcode == LNP64::ST_H || Opcode == LNP64::ST_B)
      return addStore(Inst, Operands);

    return false;
  }

  static bool addReg(MCInst &Inst, const OperandVector &Operands) {
    const LNP64Operand *Reg = getOp(Operands, 1);
    if (Operands.size() != 2 || !Reg || !Reg->isReg())
      return false;
    Inst.addOperand(MCOperand::createReg(Reg->getReg()));
    return true;
  }

  static bool addBranchTarget(MCInst &Inst, const OperandVector &Operands) {
    const LNP64Operand *Imm = getOp(Operands, 1);
    if (Operands.size() != 2 || !Imm || !Imm->isImm())
      return false;
    if (Imm->isExprValue())
      Inst.addOperand(MCOperand::createExpr(Imm->getExpr()));
    else
      Inst.addOperand(MCOperand::createImm(Imm->getImm()));
    return true;
  }

  static bool addRegImm(MCInst &Inst, const OperandVector &Operands) {
    const LNP64Operand *Reg = getOp(Operands, 1);
    const LNP64Operand *Imm = getOp(Operands, 2);
    if (Operands.size() != 3 || !Reg || !Imm || !Reg->isReg() ||
        !Imm->isImmValue())
      return false;
    if (Imm->getImm() < -32768 || Imm->getImm() > 32767)
      return false;
    Inst.addOperand(MCOperand::createReg(Reg->getReg()));
    Inst.addOperand(MCOperand::createImm(Imm->getImm()));
    return true;
  }

  static bool addRegAddress(MCInst &Inst, const OperandVector &Operands) {
    const LNP64Operand *Reg = getOp(Operands, 1);
    const LNP64Operand *Addr = getOp(Operands, 2);
    if (Operands.size() != 3 || !Reg || !Addr || !Reg->isReg() ||
        !Addr->isImm())
      return false;
    Inst.addOperand(MCOperand::createReg(Reg->getReg()));
    if (Addr->isExprValue())
      Inst.addOperand(MCOperand::createExpr(Addr->getExpr()));
    else
      Inst.addOperand(MCOperand::createImm(Addr->getImm()));
    return true;
  }

  static bool addRegReg(MCInst &Inst, const OperandVector &Operands) {
    const LNP64Operand *A = getOp(Operands, 1);
    const LNP64Operand *B = getOp(Operands, 2);
    if (Operands.size() != 3 || !A || !B || !A->isReg() || !B->isReg())
      return false;
    Inst.addOperand(MCOperand::createReg(A->getReg()));
    Inst.addOperand(MCOperand::createReg(B->getReg()));
    return true;
  }

  static bool addRegRegReg(MCInst &Inst, const OperandVector &Operands) {
    const LNP64Operand *A = getOp(Operands, 1);
    const LNP64Operand *B = getOp(Operands, 2);
    const LNP64Operand *C = getOp(Operands, 3);
    if (Operands.size() != 4 || !A || !B || !C || !A->isReg() ||
        !B->isReg() || !C->isReg())
      return false;
    Inst.addOperand(MCOperand::createReg(A->getReg()));
    Inst.addOperand(MCOperand::createReg(B->getReg()));
    Inst.addOperand(MCOperand::createReg(C->getReg()));
    return true;
  }

  static bool addRegRegRegReg(MCInst &Inst, const OperandVector &Operands) {
    const LNP64Operand *A = getOp(Operands, 1);
    const LNP64Operand *B = getOp(Operands, 2);
    const LNP64Operand *C = getOp(Operands, 3);
    const LNP64Operand *D = getOp(Operands, 4);
    if (Operands.size() != 5 || !A || !B || !C || !D || !A->isReg() ||
        !B->isReg() || !C->isReg() || !D->isReg())
      return false;
    Inst.addOperand(MCOperand::createReg(A->getReg()));
    Inst.addOperand(MCOperand::createReg(B->getReg()));
    Inst.addOperand(MCOperand::createReg(C->getReg()));
    Inst.addOperand(MCOperand::createReg(D->getReg()));
    return true;
  }

  static bool addLoad(MCInst &Inst, const OperandVector &Operands) {
    const LNP64Operand *Reg = getOp(Operands, 1);
    const LNP64Operand *Mem = getOp(Operands, 2);
    if (Operands.size() != 3 || !Reg || !Mem || !Reg->isReg() ||
        !Mem->isMem())
      return false;
    Inst.addOperand(MCOperand::createReg(Reg->getReg()));
    Inst.addOperand(MCOperand::createReg(Mem->getBaseReg()));
    Inst.addOperand(MCOperand::createImm(Mem->getImm()));
    return true;
  }

  static bool addStore(MCInst &Inst, const OperandVector &Operands) {
    return addLoad(Inst, Operands);
  }
};

} // end anonymous namespace

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64AsmParser() {
  RegisterMCAsmParser<LNP64AsmParser> X(getTheLNP64Target());
}
