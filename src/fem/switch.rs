use crate::{fem_io, Result, FEM};

#[derive(Debug, Clone, Copy)]
pub enum Switch {
    On,
    Off,
}

impl FEM {
    pub fn switch_inputs(&mut self, switch: Switch, id: Option<&[usize]>) -> &mut Self {
        for i in id
            .map(|i| i.to_vec())
            .unwrap_or_else(|| (0..self.inputs.len()).collect::<Vec<usize>>())
        {
            self.inputs.get_mut(i).and_then(|input| {
                input.as_mut().map(|input| {
                    input.iter_mut().for_each(|io| {
                        *io = match switch {
                            Switch::On => io.clone().switch_on(),
                            Switch::Off => io.clone().switch_off(),
                        };
                    })
                })
            });
        }
        self
    }
    pub fn switch_outputs(&mut self, switch: Switch, id: Option<&[usize]>) -> &mut Self {
        for i in id
            .map(|i| i.to_vec())
            .unwrap_or_else(|| (0..self.outputs.len()).collect::<Vec<usize>>())
        {
            self.outputs.get_mut(i).and_then(|output| {
                output.as_mut().map(|output| {
                    output.iter_mut().for_each(|io| {
                        *io = match switch {
                            Switch::On => io.clone().switch_on(),
                            Switch::Off => io.clone().switch_off(),
                        };
                    })
                })
            });
        }
        self
    }
    pub fn switch_input<U>(&mut self, switch: Switch) -> Option<&mut Self>
    where
        Vec<Option<fem_io::Inputs>>: fem_io::FemIo<U>,
    {
        self.in_position::<U>()
            .map(|i| self.switch_inputs(switch, Some(&[i])))
    }
    pub fn switch_output<U>(&mut self, switch: Switch) -> Option<&mut Self>
    where
        Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
    {
        self.out_position::<U>()
            .map(|i| self.switch_outputs(switch, Some(&[i])))
    }
    pub fn switch_inputs_by_name<S: Into<String>>(
        &mut self,
        names: Vec<S>,
        switch: Switch,
    ) -> Result<&mut Self> {
        for name in names {
            Box::<dyn fem_io::GetIn>::try_from(name.into())
                .map(|x| x.position(&self.inputs))
                .map(|i| i.map(|i| self.switch_inputs(switch, Some(&[i]))))?;
        }
        Ok(self)
    }
    pub fn switch_outputs_by_name<S: Into<String>>(
        &mut self,
        names: Vec<S>,
        switch: Switch,
    ) -> Result<&mut Self> {
        for name in names {
            Box::<dyn fem_io::GetOut>::try_from(name.into())
                .map(|x| x.position(&self.outputs))
                .map(|i| i.map(|i| self.switch_outputs(switch, Some(&[i]))))?;
        }
        Ok(self)
    }
}
