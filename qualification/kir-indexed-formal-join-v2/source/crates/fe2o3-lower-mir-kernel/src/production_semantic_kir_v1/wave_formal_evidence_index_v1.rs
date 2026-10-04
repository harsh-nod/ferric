// Borrowed, deterministic lookup tables for the inert archive join only.
// The caller restores its storage floor after these tables have been dropped.
struct InertJoinBlockV1<'a> {
    key: u32,
    ordinal: usize,
    block: &'a kir::BasicBlock,
}
struct InertJoinValueV1<'a> {
    key: u32,
    ordinal: usize,
    ty: &'a kir::Type,
}
struct InertJoinIndexV1<'a> {
    blocks: Vec<InertJoinBlockV1<'a>>,
    values: Vec<InertJoinValueV1<'a>>,
}

fn inert_join_sort_v1<T>(
    rows: &mut [T],
    budget: &mut Budget<'_>,
    key: impl Fn(&T) -> (u32, usize),
) -> ResultV1<()> {
    let count = rows.len();
    if count < 2 {
        return Ok(());
    }
    let height = usize::BITS as usize - (count - 1).leading_zeros() as usize;
    // Same controlled heapsort bound as kernel-ir/verification_index_v1.rs.
    // Include up to three row moves per swap and two-scalar comparisons.
    // Fewer than 2*n*height sift levels plus n root swaps fit this bound.
    let width = add(mul(size_of::<T>(), 3)?, 4)?;
    let work = mul(mul(mul(count, height)?, 4)?, width)?;
    budget.charge_work(work)?;
    for start in (0..count / 2).rev() {
        inert_join_sift_v1(rows, start, count, &key)?;
    }
    for end in (1..count).rev() {
        rows.swap(0, end);
        inert_join_sift_v1(rows, 0, end, &key)?;
    }
    Ok(())
}

fn inert_join_sift_v1<T>(
    rows: &mut [T],
    mut root: usize,
    end: usize,
    key: &impl Fn(&T) -> (u32, usize),
) -> ResultV1<()> {
    loop {
        let child = add(mul(root, 2)?, 1)?;
        if child >= end {
            return Ok(());
        }
        let right = add(child, 1)?;
        let selected = if right < end && key(&rows[child]) < key(&rows[right]) {
            right
        } else {
            child
        };
        if key(&rows[root]) >= key(&rows[selected]) {
            return Ok(());
        }
        rows.swap(root, selected);
        root = selected;
    }
}

fn inert_join_lower_bound_v1<T>(
    rows: &[T],
    wanted: u32,
    budget: &mut Budget<'_>,
    key: impl Fn(&T) -> u32,
) -> ResultV1<usize> {
    let (mut low, mut high) = (0usize, rows.len());
    while low < high {
        budget.charge_work(8)?;
        let mid = low + (high - low) / 2;
        if key(&rows[mid]) < wanted {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    budget.charge_work(4)?;
    Ok(low)
}

impl<'a> InertJoinIndexV1<'a> {
    fn build(function: &'a kir::Function, budget: &mut Budget<'_>) -> ResultV1<Self> {
        let body = function.body.as_ref().ok_or(E::Correspondence)?;
        budget.charge_work(4)?;
        let mut values = body
            .parameters
            .len()
            .min(function.signature.parameters.len());
        let mut nodes = add(body.parameters.len(), add(body.blocks.len(), 1)?)?;
        for block in &body.blocks {
            budget.charge_work(4)?;
            values = add(values, block.parameters.len())?;
            nodes = add(nodes, add(block.parameters.len(), block.operations.len())?)?;
            for operation in &block.operations {
                budget.charge_work(2)?;
                values = add(values, operation.results.len())?;
                nodes = add(nodes, operation.results.len())?;
            }
        }
        let storage = add(
            size_of::<Self>(),
            add(
                mul(body.blocks.len(), size_of::<InertJoinBlockV1<'a>>())?,
                mul(values, size_of::<InertJoinValueV1<'a>>())?,
            )?,
        )?;
        budget.charge_work(mul(add(nodes, values)?, 128)?)?;
        budget.reserve_storage(storage)?;
        let mut blocks = Vec::new();
        blocks
            .try_reserve_exact(body.blocks.len())
            .map_err(|_| E::Allocation)?;
        if blocks.capacity() != body.blocks.len() {
            return Err(E::Allocation);
        }
        let mut definitions = Vec::new();
        definitions
            .try_reserve_exact(values)
            .map_err(|_| E::Allocation)?;
        if definitions.capacity() != values {
            return Err(E::Allocation);
        }
        for (id, ty) in body.parameters.iter().zip(&function.signature.parameters) {
            definitions.push(InertJoinValueV1 {
                key: id.0,
                ordinal: definitions.len(),
                ty,
            });
        }
        for (ordinal, block) in body.blocks.iter().enumerate() {
            blocks.push(InertJoinBlockV1 {
                key: block.id.0,
                ordinal,
                block,
            });
            for value in &block.parameters {
                definitions.push(InertJoinValueV1 {
                    key: value.id.0,
                    ordinal: definitions.len(),
                    ty: &value.ty,
                });
            }
            for operation in &block.operations {
                for value in &operation.results {
                    definitions.push(InertJoinValueV1 {
                        key: value.id.0,
                        ordinal: definitions.len(),
                        ty: &value.ty,
                    });
                }
            }
        }
        if definitions.len() != values {
            return Err(E::Size);
        }
        inert_join_sort_v1(&mut blocks, budget, |row| (row.key, row.ordinal))?;
        budget.charge_work(mul(blocks.len(), 2)?)?;
        if blocks.windows(2).any(|pair| pair[0].key == pair[1].key) {
            return Err(E::Correspondence);
        }
        inert_join_sort_v1(&mut definitions, budget, |row| (row.key, row.ordinal))?;
        Ok(Self {
            blocks,
            values: definitions,
        })
    }

    fn pointer_type(
        &self,
        pointer: kir::ValueId,
        budget: &mut Budget<'_>,
    ) -> ResultV1<&'a kir::Type> {
        let index = inert_join_lower_bound_v1(&self.values, pointer.0, budget, |row| row.key)?;
        let row = self.values.get(index).ok_or(E::Correspondence)?;
        if row.key != pointer.0 {
            return Err(E::Correspondence);
        }
        // Sorting by original ordinal and taking lower_bound preserves the
        // old first-definition rule even for an unverified duplicate ValueId.
        Ok(row.ty)
    }

    fn operation(&self, location: Location, budget: &mut Budget<'_>) -> ResultV1<&'a Operation> {
        let index =
            inert_join_lower_bound_v1(&self.blocks, location.block.0, budget, |row| row.key)?;
        let row = self.blocks.get(index).ok_or(E::Correspondence)?;
        if row.key != location.block.0 {
            return Err(E::Correspondence);
        }
        row.block
            .operations
            .get(location.operation_index)
            .ok_or(E::Correspondence)
    }
}

fn inert_join_linear_work_v1(
    kir_length: usize,
    rows: usize,
    roots: usize,
    kernels: usize,
) -> ResultV1<usize> {
    // Whole-wire linear passes cover intrinsic/effect descriptors and names;
    // records and the small exact root cross-join are retained, not skipped.
    // Repeated block/value scans are replaced and charged separately above.
    mul(
        add(add(kir_length, rows)?, add(mul(roots, roots)?, kernels)?)?,
        128,
    )
}
