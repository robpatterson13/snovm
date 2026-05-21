#![allow(dead_code)]

#[derive(Copy, Clone, Debug)]
enum Instruction {
    LoadC(i64),
    Load,
    LoadA(usize),
    Store,
    StoreA(usize),
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Eq,
    Neq,
    Le,
    Leq,
    Gr,
    Geq,
    Neg,
    Not,
    Debug(DebugInstruction),
    Halt
}

#[derive(Copy, Clone, Debug)]
enum DebugInstruction {
    StackDump
}

const STACK_DUMP: Instruction = Instruction::Debug(DebugInstruction::StackDump);

type IList = Vec<Instruction>;

type ExecutionResult<R> = Result<R, ExecutionError>;
type DebugResult<R> = Result<(R, Vec<Vec<R>>), ExecutionError>;
type FinalExecutionResult = ExecutionResult<()>;

struct VM<'a> {
    pc: usize,
    stack: Vec<i64>,
    insts: &'a IList,
    debug_stack: Vec<Vec<i64>>
}

impl<'a> VM<'a> {
    pub fn new(insts: &'a IList) -> Self {
        Self { pc: 0, stack: Vec::new(), insts, debug_stack: Vec::new() }
    }
}

#[derive(PartialEq, Debug)]
enum ExecutionError {
    StackUnderflow,
    AddressDoesNotExist
}

impl VM<'_> {
    pub fn execute(mut self) -> ExecutionResult<i64> {
        self.dispatch()?;
        self.top_or(ExecutionError::StackUnderflow)
    }

    fn execute_debug(mut self) -> DebugResult<i64> {
        self.dispatch()?;
        let result = self.top_or(ExecutionError::StackUnderflow)?;
        let stack = self.debug_stack.clone();
        Ok((result, stack))
    }

    #[inline(always)]
    fn dispatch(&mut self) -> ExecutionResult<()> {
        loop {
            let op = self.fetch(self.pc);
            match op {
                Instruction::LoadC(c)  => self.do_loadc(c),
                Instruction::Pop       => self.do_pop()?,
                Instruction::Load      => self.do_load()?,
                Instruction::LoadA(i)  => self.do_loada(i)?,
                Instruction::Store     => self.do_store()?,
                Instruction::StoreA(i) => self.do_storea(i)?,

                Instruction::Add => self.do_binop(|a, b| a + b)?,
                Instruction::Sub => self.do_binop(|a, b| a - b)?,
                Instruction::Mul => self.do_binop(|a, b| a * b)?,
                Instruction::Div => self.do_binop(|a, b| a / b)?,
                Instruction::Mod => self.do_binop(|a, b| a % b)?,
                Instruction::And => self.do_binop(|a, b| a & b)?,
                Instruction::Or  => self.do_binop(|a, b| a | b)?,
                Instruction::Eq  => self.do_binop(|a, b| (a == b) as i64)?,
                Instruction::Neq => self.do_binop(|a, b| (a != b) as i64)?,
                Instruction::Le  => self.do_binop(|a, b| (a < b) as i64)?,
                Instruction::Leq => self.do_binop(|a, b| (a <= b) as i64)?,
                Instruction::Gr  => self.do_binop(|a, b| (a > b) as i64)?,
                Instruction::Geq => self.do_binop(|a, b| (a >= b) as i64)?,

                Instruction::Not => self.do_unop(|a| !a)?,
                Instruction::Neg => self.do_unop(|a| -a)?,

                Instruction::Debug(d) => self.do_debug(d),

                Instruction::Halt => return Ok(()),
            };
        }
    }

    #[inline(always)]
    fn push(&mut self, val: i64) {
        self.stack.push(val);
    }

    #[inline(always)]
    fn pop(&mut self) -> ExecutionResult<i64> {
        self.stack.pop().ok_or(ExecutionError::StackUnderflow)
    }
    
    #[inline(always)]
    fn top_or(&mut self, err: ExecutionError) -> ExecutionResult<i64> {
        self.stack.last().copied().ok_or(err)
    }

    #[inline(always)]
    fn pop2(&mut self) -> ExecutionResult<(i64, i64)> {
        let rhs = self.pop()?;
        let lhs = self.pop()?;
        Ok((lhs, rhs))
    }

    #[inline(always)]
    fn fetch(&mut self, pc: usize) -> Instruction {
        self.insts[pc]
    }

    #[inline(always)]
    fn stack_loc(&mut self, addr: usize) -> ExecutionResult<&mut i64> {
        self.stack.get_mut(addr).ok_or(ExecutionError::AddressDoesNotExist)
    }

    #[inline(always)]
    fn at_addr(&mut self, addr: usize) -> ExecutionResult<i64> {
        self.stack.get(addr).copied().ok_or(ExecutionError::AddressDoesNotExist)
    }
}

impl VM<'_> {
    #[inline(always)]
    fn do_loadc(&mut self, c: i64) {
        self.push(c);
        self.pc += 1;
    }

    #[inline(always)]
    fn do_load(&mut self) -> ExecutionResult<()> {
        let addr = self.pop()? as usize;
        let val = self.at_addr(addr)?;
        self.push(val);
        self.pc += 1;
        Ok(())
    }

    #[inline(always)]
    fn do_loada(&mut self, addr: usize) -> ExecutionResult<()> {
        let val = self.at_addr(addr)?;
        self.push(val);
        self.pc += 1;
        Ok(())
    }

    #[inline(always)]
    fn do_store(&mut self) -> ExecutionResult<()> {
        let addr = self.pop()? as usize;
        let val = self.top_or(ExecutionError::StackUnderflow)?;
        let place = self.stack_loc(addr)?;
        *place = val;
        self.pc += 1;
        Ok(())
    }

    #[inline(always)]
    fn do_storea(&mut self, addr: usize) -> ExecutionResult<()> {
        let val = self.top_or(ExecutionError::StackUnderflow)?;
        let place = self.stack_loc(addr)?;
        *place = val;
        self.pc += 1;
        Ok(())
    }

    #[inline(always)]
    fn do_pop(&mut self) -> ExecutionResult<()> {
        self.pop()?;
        self.pc += 1;
        Ok(())
    }

    #[inline(always)]
    fn do_binop<F>(&mut self, f: F) -> ExecutionResult<()>
    where
        F: Fn(i64, i64) -> i64,
    {
        let (a, b) = self.pop2()?;
        self.push(f(a, b));
        self.pc += 1;
        Ok(())
    }

    #[inline(always)]
    fn do_unop<F>(&mut self, f: F) -> ExecutionResult<()>
    where
        F: Fn(i64) -> i64,
    {
        let n = self.pop()?;
        self.push(f(n));
        self.pc += 1;
        Ok(())
    }

    #[inline(always)]
    fn do_debug(&mut self, d: DebugInstruction) {
        match d {
            DebugInstruction::StackDump => self.debug_stack.push(self.stack.clone()),
        };
        self.pc += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_vm {
        ($($insts:ident),+ => $result:expr) => {
            let insts: Vec<Instruction> = vec![$($inst),+];
            let vm = VM::new(&$insts);
            let (result, _) = vm.execute_debug().unwrap();
            assert_eq!(result, $result);
        };

        ($($inst:expr),+ => $result:expr; with stack dumps: $([$($snap_item:expr),*]),+) => {
            #[allow(unused)]
            {
                let insts: Vec<Instruction> = vec![$($inst),+];
                let vm = VM::new(&insts);
                let (result, stack) = vm.execute_debug().unwrap();
                assert_eq!(result, $result);
                let mut index = 0;
                $(
                    assert_eq!(*stack.get(index).unwrap(), vec![$($snap_item),*]);
                    index += 1;
                )*
            }
        };
    }

    #[test]
    fn eq_7() {
        test_vm!(
            Instruction::LoadC(7),
            Instruction::LoadC(1),
            STACK_DUMP,
            Instruction::Add,
            STACK_DUMP,
            Instruction::LoadC(1),
            STACK_DUMP,
            Instruction::Sub,
            STACK_DUMP,
            Instruction::Halt
            => 7;

            with stack dumps:
            [7, 1],
            [8],
            [8, 1],
            [7]
        );
    }
}
