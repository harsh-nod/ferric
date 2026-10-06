// Failure-only metadata; charge order and counter ownership remain unchanged.
const MAX_GRAPH_WORK_PATH_BYTES_V1: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GraphWorkFailureV1 {
    Overflow,
    AboveLimit,
}

impl GraphWorkFailureV1 {
    const fn detail(self) -> &'static str {
        match self {
            Self::Overflow => "uniform induction CFG analysis work overflow",
            Self::AboveLimit => "uniform induction CFG analysis exceeds its work limit",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GraphWorkContextV1 {
    DefinedCallableSummary,
    RootProjection,
}

#[derive(Debug)]
struct GraphWorkPathV1 {
    bytes: [u8; MAX_GRAPH_WORK_PATH_BYTES_V1],
    len: usize,
    truncated: bool,
}

impl GraphWorkPathV1 {
    fn new(path: &[u8]) -> Self {
        let len = path.len().min(MAX_GRAPH_WORK_PATH_BYTES_V1);
        let mut bytes = [0; MAX_GRAPH_WORK_PATH_BYTES_V1];
        bytes[..len].copy_from_slice(&path[path.len() - len..]);
        Self {
            bytes,
            len,
            truncated: path.len() > len,
        }
    }
}

impl fmt::Display for GraphWorkPathV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.bytes[..self.len].escape_ascii())
    }
}

#[derive(Debug)]
struct GraphWorkFunctionV1 {
    index: usize,
    identity: SemanticFunctionIdentityV1,
    role: SemanticFunctionRoleV1,
    source: SemanticSourceProvenanceV1,
    kernel_export: Option<CfgExportSymbolV1>,
    context: GraphWorkContextV1,
}

#[derive(Debug)]
pub(crate) struct GraphWorkDiagnosticV1 {
    reason: GraphWorkFailureV1,
    before: usize,
    amount: usize,
    sum: Option<usize>,
    path: GraphWorkPathV1,
    line: u32,
    column: u32,
    function: Option<GraphWorkFunctionV1>,
    root: Option<DeterministicProjectionRootDiagnosticV1>,
}

impl ProductionRankedProjectionErrorV1 {
    fn graph_work_v1(
        reason: GraphWorkFailureV1,
        before: usize,
        amount: usize,
        sum: Option<usize>,
        location: &'static std::panic::Location<'static>,
    ) -> Self {
        Self::GraphWorkLimit(Box::new(GraphWorkDiagnosticV1 {
            reason,
            before,
            amount,
            sum,
            path: GraphWorkPathV1::new(location.file().as_bytes()),
            line: location.line(),
            column: location.column(),
            function: None,
            root: None,
        }))
    }

    fn with_graph_work_function_v1(
        mut self,
        index: usize,
        function: &SemanticFunctionDeclV1,
        context: GraphWorkContextV1,
    ) -> Self {
        if let Self::GraphWorkLimit(diagnostic) = &mut self
            && diagnostic.function.is_none()
        {
            diagnostic.function = Some(GraphWorkFunctionV1 {
                index,
                identity: function.identity(),
                role: function.role(),
                source: function.source(),
                kernel_export: function
                    .kernel_entry()
                    .map(|entry| CfgExportSymbolV1::new(entry.export_symbol().as_bytes())),
                context,
            });
        }
        self
    }

    fn with_graph_work_body_v1(
        self,
        functions: &[SemanticFunctionDeclV1],
        body: SemanticFunctionIdV1,
    ) -> Self {
        if matches!(&self, Self::GraphWorkLimit(_))
            && let Some(function) = functions.get(body.index() as usize)
        {
            self.with_graph_work_function_v1(
                body.index() as usize,
                function,
                GraphWorkContextV1::RootProjection,
            )
        } else {
            self
        }
    }
}

impl fmt::Display for GraphWorkDiagnosticV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "semantic-to-ranked projection rejected {}; loop-graph-work-diagnostic-v1 reason={} before={} amount={} sum=",
            self.reason.detail(),
            match self.reason {
                GraphWorkFailureV1::Overflow => "overflow",
                GraphWorkFailureV1::AboveLimit => "above-limit",
            },
            self.before,
            self.amount,
        )?;
        match self.sum {
            Some(value) => write!(formatter, "{value}")?,
            None => formatter.write_str("none")?,
        }
        write!(
            formatter,
            " limit={} caller={} line={} column={} path_truncated={}",
            MAX_PROJECTED_LOOP_GRAPH_WORK_V1,
            self.path,
            self.line,
            self.column,
            self.path.truncated,
        )?;
        if let Some(function) = &self.function {
            write!(formatter, " function_index={} function_sha256=", function.index)?;
            for byte in function.identity.as_bytes() {
                write!(formatter, "{byte:02x}")?;
            }
            write!(
                formatter,
                " role={:?} source={} phase={}",
                function.role,
                source_label(function.source),
                match function.context {
                    GraphWorkContextV1::DefinedCallableSummary => "defined-callable-summary",
                    GraphWorkContextV1::RootProjection => "root-projection",
                },
            )?;
            if let Some(symbol) = &function.kernel_export {
                write!(formatter, " kernel_export={symbol}")?;
            }
        } else {
            formatter.write_str(" phase=unattributed")?;
        }
        if let Some(root) = &self.root {
            write!(formatter, "; root {root}")?;
        }
        Ok(())
    }
}
