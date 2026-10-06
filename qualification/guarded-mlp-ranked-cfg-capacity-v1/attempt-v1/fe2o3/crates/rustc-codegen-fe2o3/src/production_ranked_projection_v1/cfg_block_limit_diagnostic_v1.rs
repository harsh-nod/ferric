// Included in the ranked projector; this record cannot alter admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CfgBlockLimitSiteV1 {
    LoopSwitchInventory,
    LoopCfgWithInventory,
}

impl CfgBlockLimitSiteV1 {
    const fn label(self) -> &'static str {
        match self {
            Self::LoopSwitchInventory => "loop-switch-inventory",
            Self::LoopCfgWithInventory => "loop-cfg-with-inventory",
        }
    }
}

#[derive(Debug)]
struct CfgExportSymbolV1 {
    bytes: [u8; MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1],
    len: usize,
    truncated: bool,
}

impl CfgExportSymbolV1 {
    fn new(symbol: &[u8]) -> Self {
        let len = symbol.len().min(MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1);
        let mut bytes = [0; MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1];
        bytes[..len].copy_from_slice(&symbol[..len]);
        Self {
            bytes,
            len,
            truncated: symbol.len() > len,
        }
    }
}

impl fmt::Display for CfgExportSymbolV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.bytes[..self.len].escape_ascii())?;
        if self.truncated {
            formatter.write_str("...")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct CfgBlockLimitDiagnosticV1 {
    function: SemanticFunctionIdentityV1,
    role: SemanticFunctionRoleV1,
    source: SemanticSourceProvenanceV1,
    kernel_export: Option<CfgExportSymbolV1>,
    blocks: usize,
    limit: usize,
    site: CfgBlockLimitSiteV1,
    callable_function_index: Option<usize>,
    root: Option<DeterministicProjectionRootDiagnosticV1>,
}

impl ProductionRankedProjectionErrorV1 {
    fn cfg_block_limit_v1(function: &SemanticFunctionDeclV1, site: CfgBlockLimitSiteV1) -> Self {
        Self::CfgBlockLimit(Box::new(CfgBlockLimitDiagnosticV1 {
            function: function.identity(),
            role: function.role(),
            source: function.source(),
            kernel_export: function
                .kernel_entry()
                .map(|entry| CfgExportSymbolV1::new(entry.export_symbol().as_bytes())),
            blocks: function.blocks().len(),
            limit: MAX_RANKED_BOUNDS_BLOCKS,
            site,
            callable_function_index: None,
            root: None,
        }))
    }

    fn with_cfg_callable_context_v1(mut self, function_index: usize) -> Self {
        if let Self::CfgBlockLimit(diagnostic) = &mut self {
            diagnostic.callable_function_index = Some(function_index);
        }
        self
    }
}

impl fmt::Display for CfgBlockLimitDiagnosticV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "semantic-to-ranked projection rejected semantic CFG exceeds the ranked block limit before loop analysis; cfg-block-diagnostic-v1",
        )?;
        write!(
            formatter,
            " blocks={} limit={} reason={} site={} function_sha256=",
            self.blocks,
            self.limit,
            if self.blocks == 0 {
                "empty"
            } else {
                "above-limit"
            },
            self.site.label(),
        )?;
        for byte in self.function.as_bytes() {
            write!(formatter, "{byte:02x}")?;
        }
        write!(
            formatter,
            " role={:?}; source={}",
            self.role,
            source_label(self.source)
        )?;
        if let Some(symbol) = &self.kernel_export {
            write!(formatter, "; kernel_export={symbol}")?;
        }
        if let Some(index) = self.callable_function_index {
            write!(
                formatter,
                " phase=defined-callable-summary function_index={index}"
            )?;
        } else if self.root.is_some() {
            formatter.write_str(" phase=root-projection")?;
        } else {
            formatter.write_str(" phase=unattributed")?;
        }
        if let Some(root) = &self.root {
            write!(formatter, "; root {root}")?;
        }
        Ok(())
    }
}
