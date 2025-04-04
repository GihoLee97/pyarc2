#[cfg(all(any(target_os = "windows", target_os = "linux"), target_arch = "x86_64"))]
use libarc2::Instrument;

use libarc2::{BiasOrder, ControlMode, DataMode, ReadAt, ReadAfter, ReadType, find_ids, WaitFor, LogicLevel};
use libarc2::ArC2Error as LLArC2Error;
use libarc2::registers::{IOMask, IODir, AuxDACFn, OutputRange};
use std::borrow::Borrow;
use std::convert::{From, Into, TryInto};
use pyo3::prelude::{pymodule, pyclass, pymethods};
use pyo3::prelude::{PyAnyMethods, PyModule, PyModuleMethods, PyRefMut, PyResult, Python, PyErr, Bound};
use pyo3::{intern, exceptions, create_exception};
use numpy::{PyArray, PyReadonlyArray1, PyArrayMethods, IntoPyArray, Ix1, Ix2};


/// BiasOrder is used in combination with the multi-crosspoint pulse and
/// read operations of ArC2 (:meth:`pyarc2.Instrument.pulseread_all`,
/// :meth:`pyarc2.Instrument.pulse_all` and :meth:`Instrument.read_all`)
/// and marks the order of biasing, either column-wise or row-wise.
///
/// :var Rows: Bias rows
/// :var Cols: Bias columns
#[pyclass(name="BiasOrder", module="pyarc2")]
#[derive(Clone)]
struct PyBiasOrder{ _inner: BiasOrder }

#[allow(non_snake_case)]
#[pymethods]
impl PyBiasOrder {

    #[classattr]
    fn Rows() -> PyBiasOrder {
        PyBiasOrder { _inner: BiasOrder::Rows }
    }

    #[classattr]
    fn Cols() -> PyBiasOrder {
        PyBiasOrder { _inner: BiasOrder::Columns }
    }
}

impl From<BiasOrder> for PyBiasOrder {
    fn from(order: BiasOrder) -> Self {
        PyBiasOrder { _inner: order }
    }
}

impl From<PyBiasOrder> for BiasOrder {
    fn from(order: PyBiasOrder) -> Self {
        order._inner
    }
}

/// ReadAt is used with ramp operations of ArC2 (:meth:`pyarc2.Instrument.generate_ramp`)
/// and it signifies at what voltage should read-outs be done when requested.
/// This can be either at ``Bias`` (current ramp voltage), arbitrary voltage
/// :meth:`pyarc2.ReadAt.Arb` or ``Never`` if no read-outs are requested. The
/// latter also implies ``ReadAfter.Never``.
///
/// :var Bias: Read at current bias
/// :var Never: Never read
/// :var Arb: Read at arbitraty voltage - see :meth:`~pyarc2.ReadAt.Arb`
#[pyclass(name="ReadAt", module="pyarc2")]
#[derive(Clone)]
struct PyReadAt { _inner: ReadAt }

#[allow(non_snake_case)]
#[pymethods]
impl PyReadAt {

    #[classattr]
    fn Bias() -> PyReadAt {
        PyReadAt { _inner: ReadAt::Bias }
    }

    /// Arb(self, voltage, /)
    /// --
    ///
    /// Do read-outs at arbitrary voltage.
    ///
    /// :param f32 voltage: The value of the arbitrary voltage
    /// :return: A new ``ReadAt`` directive
    #[staticmethod]
    fn Arb(voltage: f32) -> PyReadAt {
        PyReadAt { _inner: ReadAt::Arb(voltage) }
    }

    #[classattr]
    fn Never() -> PyReadAt {
        PyReadAt { _inner: ReadAt::Never }
    }

    /// voltage(self, /)
    /// --
    ///
    /// Get the current voltage for this operation if this object was
    /// created with :meth:`pyarc2.ReadAt.Arb()`. It will raise an exception
    /// otherwise.
    ///
    /// :return: The voltage associated with this directive
    fn voltage(&self) -> PyResult<f32> {
        match self._inner {
            ReadAt::Arb(v) => Ok(v),
            _ => Err(exceptions::PyException::new_err("No voltage associated"))
        }
    }
}

impl From<ReadAt> for PyReadAt {
    fn from(readat: ReadAt) -> Self {
        PyReadAt { _inner: readat }
    }
}

impl From<PyReadAt> for ReadAt {
    fn from(readat: PyReadAt) -> Self {
        readat._inner
    }
}

/// ReadAfter is used with ramp operations of ArC2 (:meth:`pyarc2.Instrument.generate_ramp`)
/// and it signifies at when should read-outs be done. This can be either
/// after a biasing pulse (``Pulse``), after a block of biasing pulses (if more
/// that one, ``Block``), at the end of the Ramp (``Ramp``) or never (``Never``).
/// The last option also implies ``ReadAt.Never``.
///
/// :var Pulse: Read after pulsing
/// :var Ramp: Read at the end of a ramp
/// :var Block: Read after a block of indentical pulses
/// :var Never: Never read
#[pyclass(name="ReadAfter", module="pyarc2")]
#[derive(Clone)]
struct PyReadAfter { _inner: ReadAfter }

#[allow(non_snake_case)]
#[pymethods]
impl PyReadAfter {

    #[classattr]
    fn Pulse() -> PyReadAfter {
        PyReadAfter { _inner: ReadAfter::Pulse }
    }

    #[classattr]
    fn Ramp() -> PyReadAfter {
        PyReadAfter { _inner: ReadAfter::Ramp }
    }

    #[classattr]
    fn Block() -> PyReadAfter {
        PyReadAfter { _inner: ReadAfter::Block }
    }

    #[classattr]
    fn Never() -> PyReadAfter {
        PyReadAfter { _inner: ReadAfter::Never }
    }

    /// from_str(r, /)
    /// --
    ///
    /// Generate a ``ReadAfter`` object from a string value.
    ///
    /// :param str r: One of ``pulse``, ``ramp``, ``block``, ``never``
    /// :return: A new ``ReadAfter`` directive
    /// :raises ValueError: If a different value is provided
    #[staticmethod]
    fn from_str(r: &str) -> PyResult<PyReadAfter> {

        match r {
            "pulse" => Ok(PyReadAfter::Pulse()),
            "ramp" => Ok(PyReadAfter::Ramp()),
            "block" => Ok(PyReadAfter::Block()),
            "never" => Ok(PyReadAfter::Never()),
            _ => Err(exceptions::PyValueError::new_err("Unknown ReadAfter"))
        }
    }

    fn __str__(&self) -> &'static str {

        let inner = &self._inner;

        match inner {
            ReadAfter::Pulse => "pulse",
            ReadAfter::Ramp => "ramp",
            ReadAfter::Block => "block",
            ReadAfter::Never => "never"
        }
    }

    fn __repr__(&self) -> &'static str {

        let inner = &self._inner;

        match inner {
            ReadAfter::Pulse => "ReadAfter<Pulse>",
            ReadAfter::Ramp => "ReadAfter<Ramp>",
            ReadAfter::Block => "ReadAfter<Block>",
            ReadAfter::Never => "ReadAfter<Never>"
        }

    }

}

impl From<ReadAfter> for PyReadAfter {
    fn from(readafter: ReadAfter) -> Self {
        PyReadAfter { _inner: readafter }
    }
}

impl From<PyReadAfter> for ReadAfter {
    fn from(readafter: PyReadAfter) -> Self {
        readafter._inner
    }
}

/// ControlMode is used in combination with :meth:`pyarc2.Instrument.set_control_mode`
/// to switch the daughterboard operation mode. If it's :attr:`Header` then
/// connections are redirected to the header pins on the daughterboard
/// whereas if :attr:`Internal` then routing will be done internally. The first
/// option is typical when devices are connected to an external interfacing
/// system such as a probe card or manipulator. The latter is typically used
/// with on-board packages.
///
/// :var Internal: Switch to internal control
/// :var Header: Switch to external headers
#[pyclass(name="ControlMode", module="pyarc2")]
#[derive(Clone)]
struct PyControlMode{ _inner: ControlMode }

#[allow(non_snake_case)]
#[pymethods]
impl PyControlMode {

    #[classattr]
    fn Header() -> PyControlMode {
        PyControlMode { _inner: ControlMode::Header }
    }

    #[classattr]
    fn Internal() -> PyControlMode {
        PyControlMode { _inner: ControlMode::Internal }
    }
}

impl From<ControlMode> for PyControlMode {
    fn from(order: ControlMode) -> Self {
        PyControlMode { _inner: order }
    }
}

impl From<PyControlMode> for ControlMode {
    fn from(order: PyControlMode) -> Self {
        order._inner
    }
}

/// DataMode is used to signify the retrieval mode of values
/// from ArC2 memory. Typically this is used with :meth:`pyarc2.Instrument.pick_one`
/// or :meth:`pyarc2.Instrument.get_iter` to read values from memory. If
/// ``Words``/``Bits`` is selected only wordlines/bitlines will be returned.
/// Use ``All`` to return all values.
///
/// :var Words: Return values associated with wordlines
/// :var Bits: Return values associated with bitlines
/// :var All: Return all data
#[pyclass(name="DataMode", module="pyarc2")]
#[derive(Clone)]
struct PyDataMode { _inner: DataMode }

#[allow(non_snake_case)]
#[pymethods]
impl PyDataMode {

    #[classattr]
    fn Words() -> PyDataMode {
        PyDataMode { _inner: DataMode::Words }
    }

    #[classattr]
    fn Bits() -> PyDataMode {
        PyDataMode { _inner: DataMode::Bits }
    }

    #[classattr]
    fn All() -> PyDataMode {
        PyDataMode { _inner: DataMode::All }
    }
}

impl From<DataMode> for PyDataMode {
    fn from(mode: DataMode) -> Self {
        PyDataMode { _inner: mode }
    }
}

impl From<PyDataMode> for DataMode {
    fn from(mode: PyDataMode) -> Self {
        mode._inner
    }
}

#[pyclass(name="ReadType", module="pyarc2")]
#[derive(Clone)]
struct PyReadType { _inner: ReadType }

#[allow(non_snake_case)]
#[pymethods]
impl PyReadType {

    #[classattr]
    fn Current() -> PyReadType {
        PyReadType { _inner: ReadType::Current }
    }

    #[classattr]
    fn Voltage() -> PyReadType {
        PyReadType { _inner: ReadType::Voltage }
    }
}

impl From<ReadType> for PyReadType {
    fn from(rtype: ReadType) -> Self {
        PyReadType { _inner: rtype }
    }
}

impl From<PyReadType> for ReadType {
    fn from(rtype: PyReadType) -> Self {
        rtype._inner
    }
}

/// Wait condition for long running operations, such as
/// :meth:`pyarc2.Instrument.read_train`.
#[pyclass(name="WaitFor", module="pyarc2")]
#[derive(Clone)]
struct PyWaitFor { _inner: WaitFor }

#[allow(non_snake_case)]
#[pymethods]
impl PyWaitFor {

    /// Wait a specified number of nanoseconds
    ///
    /// :param int nanos: The number of nanoseconds to wait
    /// :return: A new ``WaitFor`` directive
    #[staticmethod]
    fn Nanos(nanos: u64) -> PyWaitFor {
        PyWaitFor { _inner: WaitFor::Time(std::time::Duration::from_nanos(nanos)) }
    }

    /// Wait a specified number of milliseconds
    ///
    /// :param int millis: The number of milliseconds to wait
    /// :return: A new ``WaitFor`` directive
    #[staticmethod]
    fn Millis(millis: u64) -> PyWaitFor {
        PyWaitFor { _inner: WaitFor::Time(std::time::Duration::from_millis(millis)) }
    }

    /// Wait a specified number of iterations
    ///
    /// :param int nanos: The number of iterations to wait
    /// :return: A new ``WaitFor`` directive
    #[staticmethod]
    fn Iterations(iters: usize) -> PyWaitFor {
        PyWaitFor { _inner: WaitFor::Iterations(iters) }
    }
}

impl From<WaitFor> for PyWaitFor {
    fn from(waitfor: WaitFor) -> Self {
        PyWaitFor { _inner: waitfor }
    }
}

impl From<PyWaitFor> for WaitFor {
    fn from(waitfor: PyWaitFor) -> Self {
        waitfor._inner
    }
}

/// Identifier for selecting auxiliary DAC functions. Typically used
/// with :meth:`pyarc2.Instrument.config_aux_channels`.
///
/// :var SELL: Selector circuit pulls down to this voltage
/// :var SELH: Selector circuit pulls up to this voltage
/// :var ARB1: Arbitrary power supply for DUTs - Max current 100 mA
/// :var ARB2: Arbitrary power supply for DUTs - Max current 100 mA
/// :var ARB3: Arbitrary power supply for DUTs - Max current 100 mA
/// :var ARB4: Arbitrary power supply for DUTs - Max current 100 mA
/// :var CREF: Reference voltage that the current source sources/sinks
///            current from/to. There should be a ≥3 V headroom between
///            ``CREF`` and the expected operating point of the the current
///            source. Must be within 1.5 V of ``CSET`` below.
/// :var CSET: Sets output current of the current source. The difference
///            between ``CSET`` and ``CREF`` divided by the resistor
///            selected dictates the output current. This should never
///            exceed 1.5 V. Must be within 1.5 V of ``CREF`` above.
#[pyclass(name="AuxDACFn", module="pyarc2")]
#[derive(Clone)]
struct PyAuxDACFn { _inner: AuxDACFn }

#[allow(non_snake_case)]
#[pymethods]
impl PyAuxDACFn {

    /// Selector circuit pulls down to this voltage
    #[classattr]
    fn SELL() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::SELL }
    }

    /// Selector circuit pulls up to this voltage
    #[classattr]
    fn SELH() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::SELH }
    }

    /// Arbitrary power supply for DUTs - Max current 100 mA
    #[classattr]
    fn ARB1() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::ARB1 }
    }

    /// Arbitrary power supply for DUTs - Max current 100 mA
    #[classattr]
    fn ARB2() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::ARB2 }
    }

    /// Arbitrary power supply for DUTs - Max current 100 mA
    #[classattr]
    fn ARB3() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::ARB3 }
    }

    /// Arbitrary power supply for DUTs - Max current 100 mA
    #[classattr]
    fn ARB4() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::ARB4 }
    }

    /// Reference voltage that the current source sources/sinks
    /// current from/to. There should be a ≥3 V headroom between
    /// CREF and the expected operating point of the current source
    /// Must be within 1 V of CSET.
    #[classattr]
    fn CREF() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::CREF }
    }

    /// Sets output current of the current source. The difference
    /// between CSET and CREF divided by the resistor selected
    /// dictates the output current. This should never exceed 1.5 V.
    /// Must be within 1 V of CREF.
    #[classattr]
    fn CSET() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::CSET }
    }

    /// Controls the logic level, however adjusting the logic
    /// level through the AUX DACs is discouraged; use the
    /// dedicated logic functions instead.
    #[classattr]
    fn LGC() -> PyAuxDACFn {
        PyAuxDACFn { _inner: AuxDACFn::LGC }
    }
}

impl From<AuxDACFn> for PyAuxDACFn {
    fn from(func: AuxDACFn) -> Self {
        PyAuxDACFn { _inner: func }
    }
}

impl From<PyAuxDACFn> for AuxDACFn {
    fn from(func: PyAuxDACFn) -> Self {
        func._inner
    }
}

impl From<&PyAuxDACFn> for AuxDACFn {
    fn from(func: &PyAuxDACFn) -> Self {
        func._inner
    }
}

/// Identifier for selecting the direction of GPIO pins. Typically used
/// with :meth:`pyarc2.Instrument.set_logic`. The ArC TWO GPIOs are
/// organised in 4 clusters of 8 contiguous GPIO channels. As such an IO
/// direction is shared by all 8 channels that are on the same cluster.
/// Cluster 0 is GPIO 0 to 7, Cluster 1 is GPIO 8 to 15, Cluster 2 is
/// GPIO 16 to 23 and Cluster 4 is GPIO 24 to 32.
#[pyclass(name="IODir", module="pyarc2")]
#[derive(Clone)]
struct PyIODir { _inner: IODir }

#[allow(non_snake_case)]
#[pymethods]
impl PyIODir {

    /// GPIO direction: Input
    #[classattr]
    fn IN() -> PyIODir {
        PyIODir { _inner: IODir::IN }
    }

    /// GPIO direction: Output
    #[classattr]
    fn OUT() -> PyIODir {
        PyIODir { _inner: IODir::OUT }
    }

    fn __str__(&self) -> String {
        let inner = self._inner;
        if inner == IODir::OUT {
            "IODir.OUT".to_string()
        } else {
            "IODir.IN".to_string()
        }
    }

}

impl From<IODir> for PyIODir {
    fn from(dir: IODir) -> Self {
        PyIODir { _inner: dir }
    }
}

impl From<PyIODir> for IODir {
    fn from(dir: PyIODir) -> Self {
        dir._inner
    }
}

impl From<&PyIODir> for IODir {
    fn from(dir: &PyIODir) -> Self {
        dir._inner
    }
}

/// Identifier for selecting logic levels. Typically used with
/// :meth:`pyarc2.Instrument.set_logic_level`.
///
/// :var LL1V8: 1.8 V
/// :var LL3V3: 3.3 V
/// :var LL5V: 5.0 V
#[pyclass(name="LogicLevel", module="pyarc2")]
#[derive(Clone)]
struct PyLogicLevel { _inner: LogicLevel }

#[allow(non_snake_case)]
#[pymethods]
impl PyLogicLevel {

    #[classattr]
    fn LL1V8() -> PyLogicLevel {
        PyLogicLevel { _inner:LogicLevel::LL1V8 }
    }

    #[classattr]
    fn LL3V3() -> PyLogicLevel {
        PyLogicLevel { _inner:LogicLevel::LL3V3 }
    }

    #[classattr]
    fn LL5V() -> PyLogicLevel {
        PyLogicLevel { _inner:LogicLevel::LL5V }
    }

}

impl From<LogicLevel> for PyLogicLevel {
    fn from(level: LogicLevel) -> Self {
        PyLogicLevel { _inner: level }
    }
}

impl From<PyLogicLevel> for LogicLevel {
    fn from(level: PyLogicLevel) -> Self {
        level._inner
    }
}

impl From<&PyLogicLevel> for LogicLevel {
    fn from(level: &PyLogicLevel) -> Self {
        level._inner
    }
}

#[pyclass(name="OutputRange", module="pyarc2")]
#[derive(Clone)]
struct PyOutputRange { _inner: OutputRange }

#[allow(non_snake_case)]
#[pymethods]
impl PyOutputRange {
    #[classattr]
    fn STD() -> PyOutputRange {
        PyOutputRange { _inner: OutputRange::STD }
    }

    #[classattr]
    fn EXT() -> PyOutputRange {
        PyOutputRange { _inner: OutputRange::EXT }
    }

    fn __str__(&self) -> String {
        let inner = self._inner;
        if inner == OutputRange::STD {
            "OutputRange.STD".to_string()
        } else {
            "OutputRange.EXT".to_string()
        }
    }
}

impl From<OutputRange> for PyOutputRange {
    fn from(rng: OutputRange) -> Self {
        PyOutputRange { _inner: rng }
    }
}

impl From<PyOutputRange> for OutputRange {
    fn from(pyrng: PyOutputRange) -> Self {
        pyrng._inner
    }
}

impl From<&PyOutputRange> for OutputRange {
    fn from(pyrng: &PyOutputRange) -> Self {
        pyrng._inner
    }
}

/// Catch-all exception for low-level ArC2 errors
/// --
#[pyclass(name="ArC2Error", module="pyarc2")]
struct PyArC2Error { _inner: LLArC2Error }

#[pymethods]
impl PyArC2Error {

    fn __str__(&self) -> String {
        let inner = &self._inner;
        format!("{}", inner)
    }

}

impl From<LLArC2Error> for PyArC2Error {
    fn from(err: LLArC2Error) -> Self {
        PyArC2Error { _inner: err }
    }
}

impl From<PyArC2Error> for LLArC2Error {
    fn from(err: PyArC2Error) -> Self {
        err._inner
    }
}

create_exception!(pyarc2, ArC2Error, exceptions::PyException,
    "Catch-all exception for low-level ArC2 errors. \
    There are five broad categories of low-level errors: \
    (1) FPGA communication errors, (2) Memory access errors \
    (3) Invalid device ID, (4) Inconsistent ramp errors \
    and (5) Output buffer access errors");

impl ArC2Error {
    pub fn new_exception(err: LLArC2Error) -> PyErr {
        ArC2Error::new_err(PyArC2Error { _inner: err })
    }
}

#[cfg(all(any(target_os = "windows", target_os = "linux"), target_arch = "x86_64"))]
#[pyclass(name="InstrumentLL", module="pyarc2", subclass)]
pub struct PyInstrument {
    _instrument: Instrument
}

#[cfg(all(any(target_os = "windows", target_os = "linux"), target_arch = "x86_64"))]
impl PyInstrument {

    /// Returns a reference to the underlying Instrument
    pub fn inner(&self) -> &Instrument {
        &self._instrument
    }

    /// Returns a mutable reference to the underlying Instrument
    pub fn inner_mut(&mut self) -> &mut Instrument {
        &mut self._instrument
    }
}

#[cfg(all(any(target_os = "windows", target_os = "linux"), target_arch = "x86_64"))]
#[pymethods]
impl PyInstrument {

    #[new]
    #[pyo3(signature = (id, fw, init=None))]
    fn new(id: i32, fw: &str, init: Option<bool>) -> PyResult<Self> {
        let actual_init = match init {
            Some(x) => x,
            None => true
        };
        match Instrument::open_with_fw(id, fw, true, actual_init) {
            Ok(instr) => Ok(PyInstrument { _instrument: instr }),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// delay(self, nanos, /)
    /// --
    ///
    /// Insert a delay of ``nanos`` nanoseconds in the command buffer.
    fn delay<'py>(mut slf: PyRefMut<'py, Self>, nanos: u128) -> PyResult<PyRefMut<'py, Self>> {
        match slf._instrument.add_delay(nanos) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// ground_all(self, /)
    /// --
    ///
    /// Ground all channels and revert them to arbitrary voltage operation.
    fn ground_all<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<PyRefMut<'py, Self>> {
        match slf._instrument.ground_all() {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// ground_all_fast(self, /)
    /// --
    ///
    /// Ground all channels maintaing current channel operating mode.
    fn ground_all_fast<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<PyRefMut<'py, Self>> {
        match slf._instrument.ground_all_fast() {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// connect_to_gnd(self, chans, /)
    /// --
    ///
    /// Modify previously configured channels by switching them to ground. Use
    /// an empty array to clear.
    ///
    /// :param chans: The channels to ground; this must be a numpy uint64 array or
    ///               any Iterable whose elements can be converted to uint64.
    fn connect_to_gnd<'py>(mut slf: PyRefMut<'py, Self>, chans: PyReadonlyArray1<'py, usize>)
        -> PyResult<PyRefMut<'py, Self>> {

        let slice = chans.as_slice().unwrap();
        match slf._instrument.connect_to_gnd(slice) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// gnd_add(self, chans, /)
    /// --
    ///
    /// Connect selected channels to hard ground. Unlike
    /// :meth:`~pyarc2.Instrument.connect_to_gnd` this function will not clear
    /// previously grounded channels, only add to those.
    fn gnd_add<'py>(mut slf: PyRefMut<'py, Self>, chans: PyReadonlyArray1<'py, usize>)
        -> PyResult<PyRefMut<'py, Self>> {

        let slice = chans.as_slice().unwrap();
        match slf._instrument.gnd_add(slice) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// gnd_remove(self, chans, /)
    /// --
    ///
    /// Disconnect selected channels from hard ground. Unlike
    /// :meth:`~pyarc2.Instrument.connect_to_gnd` this function will not clear
    /// previously grounded channels, only remove from those.
    fn gnd_remove<'py>(mut slf: PyRefMut<'py, Self>, chans: PyReadonlyArray1<'py, usize>)
        -> PyResult<PyRefMut<'py, Self>> {

        let slice = chans.as_slice().unwrap();
        match slf._instrument.gnd_remove(slice) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// connect_to_ac_gnd(self, chans, /)
    /// --
    ///
    /// Modify previously configured channels by switching them to AC ground. Use
    /// an empty array to clear.
    ///
    /// :param chans: The channels to ground; this must be a numpy uint64 array or
    ///               any Iterable whose elements can be converted to uint64.
    fn connect_to_ac_gnd<'py>(mut slf: PyRefMut<'py, Self>, chans: PyReadonlyArray1<'py, usize>)
        -> PyResult<PyRefMut<'py, Self>> {

        let slice = chans.as_slice().unwrap();
        match slf._instrument.connect_to_ac_gnd(slice) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// gnd_ac_add(self, chans, /)
    /// --
    ///
    /// Connect selected channels to AC ground. Unlike
    /// :meth:`~pyarc2.Instrument.connect_to_ac_gnd` this function will not clear
    /// previously grounded channels, only add to those.
    fn gnd_ac_add<'py>(mut slf: PyRefMut<'py, Self>, chans: PyReadonlyArray1<'py, usize>)
        -> PyResult<PyRefMut<'py, Self>> {

        let slice = chans.as_slice().unwrap();
        match slf._instrument.gnd_ac_add(slice) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// gnd_ac_remove(self, chans, /)
    /// --
    ///
    /// Disconnect selected channels from AC ground. Unlike
    /// :meth:`~pyarc2.Instrument.connect_to_ac_gnd` this function will not clear
    /// previously grounded channels, only remove from those.
    fn gnd_ac_remove<'py>(mut slf: PyRefMut<'py, Self>, chans: PyReadonlyArray1<'py, usize>)
        -> PyResult<PyRefMut<'py, Self>> {

        let slice = chans.as_slice().unwrap();
        match slf._instrument.gnd_ac_remove(slice) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// float_all(self, /)
    /// --
    ///
    /// Disconnect all channels.
    fn float_all<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<PyRefMut<'py, Self>> {
        match slf._instrument.float_all() {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// open_channels(self, channels, /)
    /// --
    ///
    /// Set selected channels to open, disconnecting them from the DACs. This alone is
    /// not enough to float a channel because it might be grounded previously. To
    /// properly float a channel you will have to disconnect the channels from ground
    /// first. This example floats all channels properly.
    ///
    /// >>> arc.connect_to_gnd([])  # clear grounds
    /// >>>    .open_channels(list(range(64))) # set channels to open
    /// >>>    .execute()
    ///
    /// :param channels: An array of uint64s or any Iterable with elements that can
    ///                  be converted into uint64
    fn open_channels<'py>(mut slf: PyRefMut<'py, Self>, channels: Vec<usize>) ->
        PyResult<PyRefMut<'py, Self>> {

        match slf._instrument.open_channels(&channels) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// config_channels(self, config, base, /)
    /// --
    ///
    /// Configure a set of channels at specific voltages.
    ///
    /// :param config: An array of tuples ``[(channel, voltage), ...]`` specifying
    ///                the voltage configuration.
    /// :param base: Voltage to set all channel *not* included in ``config``.
    ///              Set to ``None`` to leave them at their current state.
    #[pyo3(signature = (input, base=None))]
    fn config_channels<'py>(mut slf: PyRefMut<'py, Self>, input: Vec<(u16, f32)>, base: Option<f32>)
        -> PyResult<PyRefMut<'py, Self>> {

        match slf._instrument.config_channels(&input, base) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// config_aux_channels(self, config, base, /)
    /// --
    ///
    /// Configure the ArC2 auxiliary DACs. The AUX DACs manage signals
    /// required by the peripheral ArC2 circuitry. The only argument is an
    /// array of tuples containing a list of AUX DAC functions (tuple item #0)
    /// to set at specified voltage (tuple item #1). The available functions are
    /// specified by :class:`~pyarc2.AuxDACFn`.
    ///
    /// :param voltages: An array of tuples ``[(aux dac fn, voltage), ...]``
    fn config_aux_channels<'py>(mut slf: PyRefMut<'py, Self>, voltages: Vec<(PyAuxDACFn, f32)>)
        -> PyResult<PyRefMut<'py, Self>> {

        let rust_input: Vec<(AuxDACFn, f32)> =
            voltages.iter().map(|item| {
                let dac: AuxDACFn = (&item.0).into();
                (dac, item.1)
            }).collect();

        match slf._instrument.config_aux_channels(&rust_input) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// config_selectors(self, selectors, /)
    /// --
    ///
    /// Configure the ArC2 selector circuits. The array is a list of selector
    /// channels (0..31) to toggle high. The rest of the selectors will be
    /// toggled low. This function does not configure the voltage of the low
    /// and high levels, as this must be done with
    /// :meth:`~pyarc2.Instrument.config_aux_channels` and setting the high and
    /// low voltages through the :class:`~pyarc2.AuxDACFn` ``SELH``/``SELL`` variables.
    /// Perhaps unintuitively the high voltage can actually be configured to be lower
    /// than low although the usefulness of this choice is questionable.
    ///
    /// >>> # the following will set the low and high voltage for selectors to
    /// >>> # 0.0 and 3.3 V respectively and toggle selectors 9 and 12 to high.
    /// >>> arc2.config_aux_channels([(AuxDACFn.SELL, 0.0), (AuxDACFn.SELH, 3.3)])
    /// >>>     .config_selectors([9, 12])
    /// >>>     .execute()
    ///
    /// Voltage configuration need only be provided once as it is sticky.
    ///
    /// :param selectors: An array of selectors to toggle high. Use an empty array
    ///                   to clear all selectors
    fn config_selectors<'py>(mut slf: PyRefMut<'py, Self>, selectors: Vec<usize>)
        -> PyResult<PyRefMut<'py, Self>> {

        match slf._instrument.config_selectors(&selectors) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// read_one(self, low, high, vread, /)
    /// --
    ///
    /// Perform a current read between the specified channels. The low
    /// channel will be biased with `-vread` and the current will be read
    /// from the high channel.
    ///
    /// :param int low: The low voltage channel
    /// :param int high: The high voltage channel
    /// :param float vread: The voltage to read at
    /// :return: The current between the specified crosspoints at ``vread``
    /// :rtype: float
    fn read_one(&mut self, low: usize, high: usize, vread: f32) -> f32 {
        self._instrument.read_one(low, high, vread).unwrap()
    }

    /// read_slice(self, chan, vread, /)
    /// --
    ///
    /// Read all the values which have ``chan`` as the low channel. If ``chan`` is
    /// between 0 and 15 or 32 and 47 (inclusive) this will correspond to a
    /// row read at ``vread`` in a standard 32×32 array. Otherwise it's a column
    /// read.
    ///
    /// :param int chan: The low voltage channel
    /// :param float vread: The voltage to read at
    /// :return: The current of each individual channel along the ``chan`` line sinked
    ///          at ``chan``
    /// :rtype: A numpy f32 array
    fn read_slice<'py>(&mut self, py: Python<'py>, chan: usize, vread: f32) -> Bound<'py, PyArray<f32, Ix1>> {
        let array = self._instrument.read_slice(chan, vread).unwrap();
        array.into_pyarray_bound(py)
    }

    /// read_slice_masked(self, chan, mask, vread, /)
    /// --
    ///
    /// Read all the masked high channels which have ``chan`` as the low channel.
    /// If ``chan`` is between 0 and 15 or 32 and 47 (inclusive) this will
    /// correspond to a row read at ``vread`` in a standard 32×32 array. Otherwise
    /// it's a column read.
    ///
    /// :param int chan: The low voltage channel
    /// :param mask: The high-voltage channels. This must be a numpy uint64 array or
    ///              any other Iterable whose elements can be converted to uint64
    /// :param float vread: The voltage to read at
    /// :return: The current of each selected channel along the ``chan`` line sinked
    ///          at ``chan``; unselected channels will default to ``NaN``
    /// :rtype: A numpy f32 array
    fn read_slice_masked<'py>(&mut self, py: Python<'py>, chan: usize,
        mask: PyReadonlyArray1<'py, usize>, vread: f32) -> Bound<'py, PyArray<f32, Ix1>> {

        let maskslice = mask.as_slice().unwrap();
        let res = self._instrument.read_slice_masked(chan, maskslice, vread).unwrap();

        res.into_pyarray_bound(py)
    }

    /// mac(self, inp_chans, out_chans, /)
    /// --
    ///
    /// Drive arbitrary voltage to the input channels and simultaneously read current from output channels.
    ///
    /// :param list inp_chans: A list of doubles containing the configuration of the selected
    ///                    channels in the form ``(chan number, input voltage)``
    /// :param out_chans: An array of uint64s or any Iterable with elements that can
    ///                  be converted into uint64
    /// :return: The current of each individual channel along the ``out_chans`` line
    /// :rtype: A numpy f32 array
    fn mac<'py>(&mut self, py: Python<'py>, inp_chans: Vec<(usize, f32)>,
        out_chans: Vec<usize>) -> Bound<'py, PyArray<f32, Ix1>> {

        let res = self._instrument.mac(&inp_chans, &out_chans).unwrap();

        res.into_pyarray_bound(py)
    }

    /// read_all(self, vread, order, /)
    /// --
    ///
    /// Read all the available crosspoints at the specified voltage. This can be
    /// done by biasing either rows or columns.
    ///
    /// :param float vread: The read-out voltage
    /// :param order: A variant of :class:`pyarc2.BiasOrder` denoting which rows are
    ///              biased during read-out.
    /// :return: An 32×32 array containing the current measured on each individual
    ///          cronsspoint
    /// :rtype: A numpy (2, 2) f32 ndarray
    fn read_all<'py>(&mut self, py: Python<'py>, vread: f32, order: PyBiasOrder) -> Bound<'py, PyArray<f32, Ix2>> {

        let data = self._instrument.read_all(vread, order.into()).unwrap();
        let array = data.into_pyarray_bound(py);
        array.borrow().reshape((32, 32)).unwrap()
    }

    /// read_slice_open_deferred(self, highs, ground_after, /)
    /// --
    ///
    /// Perform an open current measurement along the specified channels without immediately
    /// returning a value. This can be used in an calling sequence that involves multiple
    /// steps without flushing the internal command buffer.
    #[pyo3(signature = (highs, ground_after=None))]
    fn read_slice_open_deferred<'py>(mut slf: PyRefMut<'py, Self>, highs: PyReadonlyArray1<'py, usize>,
        ground_after: Option<bool>) -> PyResult<PyRefMut<'py, Self>> {

        let slice = highs.as_slice().unwrap();
        let ground = ground_after.unwrap_or(true);

        match slf._instrument.read_slice_open_deferred(slice, ground) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// read_slice_open(self, highs, ground_after, /)
    /// --
    ///
    /// Perform an open current measurement along the specified channels. This method does not do
    /// any bias-related setup. It's up to the user to setup channels before performing the read.
    /// If ``ground_after`` is True or None a ground operation will additionally be issued
    /// post-read.
    ///
    /// :param highs: The channels to read currents from. This must be a numpy uint64 or
    ///               an Iterable whose elements can be converted to uint64
    /// :param bool ground_after: Whether channels will be grounded automatically after
    ///                           current is read
    /// :rtype: A numpy f32 array
    #[pyo3(signature = (highs, ground_after=None))]
    fn read_slice_open<'py>(&mut self, py: Python<'py>, highs: PyReadonlyArray1<'py, usize>,
        ground_after: Option<bool>) -> Bound<'py, PyArray<f32, Ix1>> {

        let slice = highs.as_slice().unwrap();
        let ground = ground_after.unwrap_or(true);

        self._instrument.read_slice_open(slice, ground).unwrap().into_pyarray_bound(py)
    }

    /// pulse_one(self, low, high, voltage, nanos, /)
    /// --
    ///
    /// Apply a pulse between the specified crosspoints with specified voltage and
    /// pulse width (in nanoseconds).
    ///
    /// :param int low: The low voltage channel (typ. grounded)
    /// :param int high: The high voltage channel
    /// :param float voltage: The pulsing voltage
    /// :param int nanos: The duration of the pulse in nanoseconds
    fn pulse_one<'py>(mut slf: PyRefMut<'py, Self>, low: usize, high: usize, voltage: f32, nanos: u128)
        -> PyResult<PyRefMut<'py, Self>> {

        match slf._instrument.pulse_one(low, high, voltage, nanos) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// pulse_slice(self, chan, voltage, nanos, /)
    /// --
    ///
    /// Apply a pulse to a row or column using ``chan`` as the low channel
    /// with specified voltage and pulse width (in nanoseconds).
    ///
    /// :param int chan: The low voltage channel (typ. grounded)
    /// :param float voltage: The pulsing voltage
    /// :param int nanos: The duration of the pulse in nanoseconds
    fn pulse_slice<'py>(mut slf: PyRefMut<'py, Self>, chan: usize, voltage: f32, nanos: u128)
        -> PyResult<PyRefMut<'py, Self>> {

        match slf._instrument.pulse_slice(chan, voltage, nanos) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// pulse_slice_masked(self, chan, mask, voltage, nanos, /)
    /// --
    ///
    /// Apply a pulse to a row or column using ``chan`` as the low channel with
    /// specified voltage and pulse width (in nanoseconds) and also limit the
    /// high channels to those specified by the mask array.
    ///
    /// :param int chan: The low voltage channel
    /// :param float voltage: The pulsing voltage
    /// :param int nanos: The pulse duration in nanoseconds
    /// :param mask: A numpy array or Iterable with the high voltage channels; same
    ///              semantics as :meth:`~pyarc2.Instrument.read_slice_masked`
    fn pulse_slice_masked<'py>(mut slf: PyRefMut<'py, Self>, chan: usize, voltage: f32, nanos: u128,
        mask: PyReadonlyArray1<'py, usize>)
        -> PyResult<PyRefMut<'py, Self>> {

        let actual_mask = mask.as_slice().unwrap();

        match slf._instrument.pulse_slice_masked(chan, actual_mask, voltage, nanos) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// pulse_slice_fast_open(self, chans, timings, preset_state, /)
    /// --
    ///
    /// Apply a sub-500 ms pulse to all specified channels.  This differs from
    /// :meth:`~pyarc2.Instrument.pulse_slice` as it does not expect a low potential channel as the
    /// "receiving" end.  When ``preset_state`` is true the state of high speed drivers will be
    /// initialised before the actual pulsing sequence begins. ``chans`` is a list of tuples -
    /// ``[(chan number, pulse voltage, normal voltage), ...]`` - and ``cl_nanos`` contains the
    /// timings per cluster. ``cl_nanos`` *MUST* be 8-items long or a ``ValueError`` will be
    /// raised.  A cluster timing can be ``None`` which means that the channels of this cluster
    /// won't be pulsed at all.  This method will throw an error if a channel is included in the
    /// ``chans`` list but the channel's corresponding cluster timing, ``int(chan/8)``, is set to
    /// ``None``.
    ///
    /// Be aware that the transition of voltages on channels belonging to the same cluster *must*
    /// be identical, which effectively means that there can be only one type of transition from
    /// pulse voltage to normal voltage per 8 consecutive channels (so high → low or low → high).
    /// If mixed transitions are provided an error will be raised.
    ///
    /// Also note that this function uses only the high speed drivers of ArC2 for pulse generation.
    /// As such the maximum pulse width is limited to 500 ms. If you want longer pulses you can get
    /// the same behaviour with a chain of :meth:`~pyarc2.Instrument.config_channels()` and
    /// :meth:`~pyarc2.Instrument.delay()` instructions.
    ///
    /// :param list chans: A list of triples containing the configuration of the selected
    ///                    channels in the form ``(chan number, pulse voltage, normal voltage)``
    /// :param list cl_nanos: A list of 8 values containing the cluster timings (pulse widths)
    ///                       in nanoseconds - can be ``None`` which will effectively skip the
    ///                       cluster altogether
    /// :param bool preset_state: Whether the high speed drivers should be preloaded before
    ///                           the actual pulsing
    ///
    /// :raises ValueError: When the timings list contains more or fewer than 8 elements
    /// :raises ~pyarc2.ArC2Error: When incorrect timings or incompatible channel polarities
    ///                           are supplied.
    fn pulse_slice_fast_open<'py>(mut slf: PyRefMut<'py, Self>, chans: Vec<(usize, f32, f32)>,
        cl_nanos: Vec<Option<u128>>, preset_state: bool) -> PyResult<PyRefMut<'py, Self>> {

        if cl_nanos.len() != 8 {
            return Err(exceptions::PyValueError::new_err("Need 8 arguments for cluster timings"));
        }

        let actual_cl_nanos: [Option<u128>; 8] = cl_nanos[0..8].try_into()?;

        match slf._instrument.pulse_slice_fast_open(&chans, &actual_cl_nanos, preset_state) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// pulse_all(self, voltage, nanos, order, /)
    /// --
    ///
    /// Pulse all crosspoints in the array, by biasing either rows or columns.
    ///
    /// :param float voltage: The pulsing voltage
    /// :param int nanos: The pulse duration in nanoseconds
    /// :param order: A variant of :class:`pyarc2.BiasOrder`
    fn pulse_all<'py>(mut slf: PyRefMut<'py, Self>, voltage: f32, nanos: u128, order: PyBiasOrder)
        -> PyResult<PyRefMut<'py, Self>> {

        match slf._instrument.pulse_all(voltage, nanos, order.into()) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// pulseread_one(self, low, high, vpulse, nanos, vread, /)
    /// --
    ///
    /// Pulse and then read a crosspoint. Same semantics as ``pulse_one`` and
    /// ``read_one`` apply.
    ///
    /// :param int low: The low voltage channel
    /// :param int high: The high voltage channel
    /// :param float vpulse: The pulsing voltage
    /// :param int nanos: The pulse duration in nanoseconds
    /// :param float vread: The voltage to read at after pulsing
    /// :return: The current between the specified crosspoints at ``vread`` after
    ///          a ``vpulse`` pulse of ``nanos`` duration has been applied
    /// :rtype: float
    fn pulseread_one(&mut self, low: usize, high: usize, vpulse: f32, nanos: u128, vread: f32) -> f32 {
        self._instrument.pulseread_one(low, high, vpulse, nanos, vread).unwrap()
    }

    /// pulseread_slice(self, chan, vpulse, nanos, vread, /)
    /// --
    ///
    /// Pulse and then read a row/column. Same semantics as
    /// :meth:`~pyarc2.Instrument.pulse_slice` and
    /// :meth:`~pyarc2.Instrument.read_slice` apply.
    ///
    /// :param int chan: The low voltage channel
    /// :param float vpulse: The pulsing voltage
    /// :param int nanos: The pulse duration in nanoseconds
    /// :param float vread: The voltage to read at
    /// :return: The current of each individual channel along the ``chan`` line sinked
    ///          at ``chan`` after a ``vpulse`` pulse of ``nanos`` duration has been
    ///          applied
    /// :rtype: A numpy f32 array
    fn pulseread_slice<'py>(&mut self, py: Python<'py>, chan: usize, vpulse: f32,
        nanos: u128, vread: f32) -> Bound<'py, PyArray<f32, Ix1>> {

        let data = self._instrument.pulseread_slice(chan, vpulse, nanos, vread).unwrap();
        data.into_pyarray_bound(py)
    }

    /// pulseread_slice_masked(self, chan, mask, vpulse, nanos, vread, /)
    /// --
    ///
    /// Pulse and read specified high channels that have ``chan`` as low potential
    /// channel. Same semantics as :meth:`~pyarc2.Instrument.pulse_slice_masked`
    /// and :meth:`~pyarc2.Instrument.read_slice_masked` apply.
    ///
    /// :param int chan: The low voltage channel
    /// :param mask: A numpy array or Iterable with the high-voltage channels.
    ///              Elements must be uint64 or convertible to uint64
    /// :param float vpulse: The pulsing voltage
    /// :param int nanos: The pulse duration in nanoseconds
    /// :param float vread: The voltage to read at
    /// :return: The current of each selected channel along the ``chan`` line sinked
    ///          at ``chan``; unselected channels will default to ``NaN``
    /// :rtype: A numpy f32 array
    fn pulseread_slice_masked<'py>(&mut self, py: Python<'py>, chan: usize,
        mask: PyReadonlyArray1<'py, usize>, vpulse: f32, nanos: u128,
        vread: f32) -> Bound<'py, PyArray<f32, Ix1>> {

        let slice = mask.as_slice().unwrap();
        let data = self._instrument.pulseread_slice_masked(chan, slice, vpulse, nanos, vread)
            .unwrap();
        data.into_pyarray_bound(py)
    }

    /// pulseread_all(self, vpulse, nanos, vread, order, /)
    /// --
    ///
    /// Pulse and read all the crosspoints. Same semantics as
    /// :meth:`~pyarc2.Instrument.pulse_all` and :meth:`~pyarc2.Instrument.read_all`
    /// apply.
    ///
    /// :param float vpulse: The pulsing voltage
    /// :param int nanos: The pulse duration in nanoseconds
    /// :param float vread: The read-out voltage
    /// :param order: A variant of :class:`pyarc2.BiasOrder` denoting which rows are
    ///              biased during read-out.
    /// :return: An 32×32 array containing the current measured on each individual
    ///          cronsspoint
    /// :rtype: A numpy (2, 2) f32 ndarray
    fn pulseread_all<'py>(&mut self, py: Python<'py>, vpulse: f32, nanos: u128,
        vread: f32, order: PyBiasOrder) -> Bound<'py, PyArray<f32, Ix2>> {

        let data = self._instrument.pulseread_all(vpulse, nanos, vread, order.into())
            .unwrap();
        let array = data.into_pyarray_bound(py);
        array.borrow().reshape((32, 32)).unwrap()

    }

    /// vread_channels(self, chans, averaging, /)
    /// --
    ///
    /// Do a voltage read across selected channels flushing the internal
    /// command buffer and immediately returning a value.
    ///
    /// :param chans: A uint64 numpy array or Iterable of the channels to
    ///               read voltage from
    /// :param bool averaging: Whether averaging should be used
    ///
    /// :rtype: An array with the voltage readings of the selected channels
    ///         in ascending order
    fn vread_channels<'py>(&mut self, chans: PyReadonlyArray1<'py, usize>, averaging: bool) -> Vec<f32> {
        let slice = chans.as_slice().unwrap();
        self._instrument.vread_channels(slice, averaging).unwrap()
    }

    /// vread_channels_deferred(self, channels, averaging, /)
    /// --
    ///
    /// Do a voltage read across selected channels without immediately returning
    /// a value. This can be used in a calling sequence that involves multiple
    /// steps without immediately flushing the internal command buffer.
    ///
    /// :param chans: A uint64 numpy array or Iterable of the channels to
    ///               read voltage from
    /// :param bool averaging: Whether averaging should be used
    fn vread_channels_deferred<'py>(mut slf: PyRefMut<'py, Self>, chans: PyReadonlyArray1<'py, usize>, averaging: bool) ->
        PyResult<PyRefMut<'py, Self>> {

        let slice = chans.as_slice().unwrap();

        match slf._instrument.vread_channels_deferred(slice, averaging) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// execute(self, /)
    /// --
    ///
    /// Write everything in the command buffer to the instrument. This will cause ArC2
    /// to start executing the instructions provided.
    fn execute<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<PyRefMut<'py, Self>> {
        match slf._instrument.execute() {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// busy(self, /)
    /// --
    ///
    /// Returns `True` if the command buffer has not been consumed.
    fn busy(&self) -> bool {
        self._instrument.busy()
    }

    /// wait(self, /)
    /// --
    ///
    /// Block until the instrument has executed its command buffer.
    fn wait(&self) {
        self._instrument.wait();
    }

    /// set_control_mode(self, mode, /)
    /// --
    ///
    /// Set daughterboard control mode either as Internal or Header
    ///
    /// :param mode: A variant of :class:`pyarc2.ControlMode`
    fn set_control_mode<'py>(mut slf: PyRefMut<'py, Self>, mode: PyControlMode) -> PyResult<PyRefMut<'py, Self>> {
        match slf._instrument.set_control_mode(mode.into()) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// set_logic(self, channel_mask, enable, /)
    /// --
    ///
    /// Set the digital I/Os specified by ``mask`` to either high (when ``enable`` is ``True``)
    /// or low (when ``enable`` is ``False``). An :meth:`~pyarc2.Instrument.execute` is
    /// required to actually load the configuration.
    ///
    /// :param int mask: A ``u32`` bitmask of the channels this function will be applied to
    /// :param cl0: Direction of GPIO cluster 0 (channels 0–7). Defaults to output.
    /// :param cl1: Direction of GPIO cluster 1 (channels 8–15). Defaults to output.
    /// :param cl2: Direction of GPIO cluster 2 (channels 16–23). Defaults to output.
    /// :param cl3: Direction of GPIO cluster 3 (channels 24–32). Defaults to output.
    #[pyo3(signature = (mask, cl0=None, cl1=None, cl2=None, cl3=None))]
    fn set_logic<'py>(mut slf: PyRefMut<'py, Self>, mask: u32,
        cl0: Option<PyIODir>, cl1: Option<PyIODir>, cl2: Option<PyIODir>, cl3: Option<PyIODir>)
        -> PyResult<PyRefMut<'py, Self>> {

        let mask = IOMask::from_vals(&[mask]);

        let actual_cl0 = match cl0 {
            Some(x) => x._inner,
            None => IODir::OUT
        };

        let actual_cl1 = match cl1 {
            Some(x) => x._inner,
            None => IODir::OUT
        };

        let actual_cl2 = match cl2 {
            Some(x) => x._inner,
            None => IODir::OUT
        };

        let actual_cl3 = match cl3 {
            Some(x) => x._inner,
            None => IODir::OUT
        };

        match slf._instrument.set_logic(actual_cl0, actual_cl1, actual_cl2, actual_cl3, &mask) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// set_logic_level(self, level, /)
    /// --
    ///
    /// Configure the ArC2 digital IO logic level. The available logic levels
    /// are specified by :class:`~pyarc2.LogicLevel`.
    ///
    /// :param level: An instance of :class:`~pyarc2.LogicLevel`.
    fn set_logic_level<'py>(mut slf: PyRefMut<'py, Self>, level: PyLogicLevel)
        -> PyResult<PyRefMut<'py, Self>> {

        match slf._instrument.set_logic_level(level.into()) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// set_channel_range(self, chans, level, /)
    /// --
    ///
    /// Set the range of the ArC2 analogue channels. The available ranges
    /// are defined by :class:`~pyarc2.OutputRange`
    ///
    /// :param chans: A list of analogue channel indices to change range
    /// :param rng: The range to set the channels to. Standard range is
    ///             ±10 V, extended range is ±20 V.
    fn set_channel_range<'py>(mut slf: PyRefMut<'py, Self>, chans: PyReadonlyArray1<'py, usize>, rng: PyOutputRange)
        -> PyResult<PyRefMut<'py, Self>> {
        let slice = chans.as_slice().unwrap();
        match slf._instrument.set_channel_range(slice, &rng.into()) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// currents_from_address(self, addr, channels, /)
    /// --
    ///
    /// Read current values from specific address segment. This will return all
    /// the channel values stored in the segment in ascending channel order
    ///
    /// :param int addr: The memory address to read currents from
    /// :param chans: The channel numbers to retrieve values from. This must be a
    ///               numpy uint64 array or any Iterable whose elements can be
    ///               converted to uint64.
    /// :return: An array with the currents of selected channels; unselected channels
    ///          will be replaced with ``Nan``
    /// :rtype: A numpy f32 array
    fn currents_from_address<'py>(&self, py: Python<'py>, addr: u32,
        chans: PyReadonlyArray1<'py, usize>) -> PyResult<Bound<'py, PyArray<f32, Ix1>>> {

        match self._instrument.currents_from_address(addr, chans.as_slice().unwrap()) {
            Ok(result) => Ok(result.into_pyarray_bound(py)),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// word_currents_from_address(self, addr, channels, /)
    /// --
    ///
    /// Read all word current values from specific address segment. This will return all
    /// word-related values stored in the segment in ascending channel order
    ///
    /// :param int addr: The memory address to read currents from
    /// :return: An array with the currents of all wordline-corresponding channels
    /// :rtype: A numpy f32 array
    fn word_currents_from_address<'py>(&self, py: Python<'py>, addr: u32)
        -> PyResult<Bound<'py, PyArray<f32, Ix1>>> {
        match self._instrument.word_currents_from_address(addr) {
            Ok(result) => Ok(result.into_pyarray_bound(py)),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// bit_currents_from_address(self, addr, /)
    /// --
    ///
    /// Read all bit current values from specific address segment. This will return all
    /// bit-related values stored in the segment in ascending channel order
    ///
    /// :param int addr: The memory address to read currents from
    /// :return: An array with the currents of all bitline-corresponding channels
    /// :rtype: A numpy f32 array
    fn bit_currents_from_address<'py>(&self, py: Python<'py>, addr: u32)
        -> PyResult<Bound<'py, PyArray<f32, Ix1>>> {
        match self._instrument.bit_currents_from_address(addr) {
            Ok(result) => Ok(result.into_pyarray_bound(py)),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }


    /// generate_ramp(self, low, high, vstart, vstep, vstop, pw, inter, npulse, readat, readafter,
    /// /)
    /// --
    ///
    /// Initiate a ramp operation in ArC2. This will spawn a background process that bias
    /// the selected ``low`` and ``high`` channels based on the parameters specified.
    /// Please note that results must be retrieved from ArC2 using the
    /// :meth:`~pyarc2.Instrument.get_iter` method which iterates and expends the internal
    /// output buffer. Alternatively :meth:`~pyarc2.Instrument.pick_one` will return the
    /// first available result.
    ///
    /// :param int low: The low voltage channel (typ. grounded)
    /// :param int high: The high voltage channel
    /// :param float vstart: The initial voltage of the ramp
    /// :param float vstep: The increment (or decrement) of every ramp step
    /// :param float vstop: The final voltage step
    /// :param int pw_nanos: The pulse width for each individual pulse in nanoseconds
    /// :param int inter_nanos: Delay between consecutive pulses in nanoseconds
    /// :param int num_pulses: Number of pulses per individual voltage step
    /// :param read_at: Variant of :class:`pyarc2.ReadAt` denoting the voltage (if any)
    ///                 of read-out operations (if any)
    /// :param read_after: Variant of :class:`pyarc2.ReadAfter` denoting when read-outs
    ///                    will be done (if ever)
    fn generate_ramp<'py>(mut slf: PyRefMut<'py, Self>, low: usize, high: usize,
        vstart: f32, vstep: f32, vstop: f32,
        pw_nanos: u128, inter_nanos: u128, num_pulses: usize,
        read_at: PyReadAt, read_after: PyReadAfter) -> PyResult<PyRefMut<'py, Self>> {

        match slf._instrument.generate_ramp(low, high, vstart, vstep, vstop,
            pw_nanos, inter_nanos, num_pulses, read_at.into(),
            read_after.into()) {
            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

    /// generate_read_train(self, lows, highs, vread, nreads, inter_nanos, ground, /)
    /// --
    ///
    /// Initiate a current read train. This will queue instructions that will read
    /// currents between ``lows`` and ``highs`` channels. The low channels can be
    /// ``None`` which means that ArC 2 will do open reads from the high channels.
    /// Results must be retrieved with an iterator as described in
    /// :meth:`~pyarc2.Instrument.generate_ramp`.
    ///
    /// :param lows: An array of unsinged integers denoting the low channels or ``None``
    ///              for unbiased open reads
    /// :param highs: An array of unsigned integeres denoting the channels to read
    ///               current from
    /// :param float vread: Read-out voltage
    /// :param int nreads: Number of current reads to perform
    /// :param int inter_nanos: Delay (in ns) between subsequent reads; can be 0
    /// :param bool ground: Whether to ground high and low channels after the
    ///                     operation
    #[pyo3(signature = (lows, highs, vread, nreads, inter_nanos, ground))]
    fn generate_read_train<'py>(mut slf: PyRefMut<'py, Self>,
        lows: Option<PyReadonlyArray1<'py, usize>>, highs: PyReadonlyArray1<'py, usize>,
        vread: f32, nreads: usize, inter_nanos: u128, ground: bool)
        -> PyResult<PyRefMut<'py, Self>> {

            let high_chans = highs.as_slice().unwrap();
            let low_chans = match lows {
                Some(chans) => {
                    let c = chans.as_slice().unwrap();
                    let mut vec = Vec::with_capacity(c.len());
                    vec.extend_from_slice(c);
                    vec
                }
                None => vec![]
            };

            match slf._instrument.generate_read_train(&low_chans, high_chans,
                vread, nreads, inter_nanos, ground) {

                Ok(_) => Ok(slf),
                Err(err) => Err(ArC2Error::new_exception(err))

            }
    }

    /// generate_vread_train(self, uchans, averaging, /)
    /// --
    ///
    /// Initiate a voltage read train. This will queue instructions that will read
    /// voltages from all ``uchans`` channels. Results must be retrieved with an
    /// iterator as described in :meth:`~pyarc2.Instrument.generate_ramp`.
    ///
    /// :param uchans: An array of unsinged integers to read voltages from
    /// :param bool averaging: Whether to perform averaged (``True``) or one-shot reads
    ///                        (``False``).
    fn generate_vread_train<'py>(mut slf: PyRefMut<'py, Self>,
        uchans: PyReadonlyArray1<'py, usize>, averaging: bool,
        npulses: usize, inter_nanos: u128) -> PyResult<PyRefMut<'py, Self>> {

        let chans = uchans.as_slice().unwrap();

        match slf._instrument.generate_vread_train(chans, averaging, npulses,
            inter_nanos) {

            Ok(_) => Ok(slf),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// read_train(self, low, high, vread, interpulse, condition, /)
    /// --
    ///
    /// Perform a retention-like operation based on subsequent number of read
    /// pulses which can be separated by `interpulse` nanoseconds.
    ///
    /// :param int low: The low voltage channel (typ. grounded)
    /// :param int high: The high voltage channel
    /// :param float vread: Read-out voltage
    /// :param int interpulse: Delay between consecutive read-outs in nanoseconds
    /// :param condition: Variant of :class:`pyarc2.WaitFor` denoting the termination
    ///                   condition for this read train
    #[pyo3(signature = (low, high, vread, interpulse, preload, condition))]
    fn read_train<'py>(mut slf: PyRefMut<'py, Self>, low: usize, high: usize,
        vread: f32, interpulse: u64, preload: Option<f32>, condition: PyWaitFor)
        -> PyResult<()> {

        match slf._instrument.read_train(low, high, vread, interpulse as u128,
            preload, condition.into()) {
            Ok(_) => Ok(()),
            Err(err) => Err(ArC2Error::new_exception(err))
        }
    }

    /// pick_one(self, mode, /)
    /// --
    ///
    /// Read a slab of data from the internal long operation buffer. This clears
    /// the memory area after reading.
    ///
    /// :param mode: A variant of :class:`pyarc2.DataMode`.
    /// :param rtype: A variant of :class:`pyarc2.ReadType`. Use `Current` to
    ///               decode values into current readings or `Voltage` to
    ///               decode them into voltage readings
    /// :return: An array with 64 (if ``DataMode.All``) or 32 (for any other
    ///          ``DataMode`` variant) floats
    /// :rtype: An f32 numpy array
    fn pick_one<'py>(&mut self, py: Python<'py>, mode: PyDataMode, rtype: PyReadType) ->
        PyResult<Option<Bound<'py, PyArray<f32, Ix1>>>> {

        let mode: DataMode = mode.into();
        let rtype: ReadType = rtype.into();

        match self._instrument.pick_one(mode, rtype) {
            Ok(data_opt) => {
                match data_opt {
                    Some(data) => {
                        let array = data.into_pyarray_bound(py);
                        Ok(Some(array))
                    },
                    None => Ok(None)
                }
            },
            Err(err) => Err(ArC2Error::new_exception(err))
        }

    }

}

#[pymodule]
fn pyarc2(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {

    /// find_ids()
    /// --
    ///
    /// Find all available ArC2 devices. This will return a list
    /// with all discovered ids.
    ///
    /// >>> from pyarc2 import find_ids
    /// >>> ids = find_ids()
    /// >>> print(ids)
    /// [0, 1]
    #[pyfn(m)]
    #[pyo3(name="find_ids")]
    fn py_find_ids(_py: Python) -> PyResult<Vec<i32>> {
        match find_ids() {
            Ok(ids) => { Ok(ids) },
            Err(err) => { Err(ArC2Error::new_exception(err)) }
        }
    }

    #[cfg(all(any(target_os = "windows", target_os = "linux"), target_arch = "x86_64"))]
    m.add_class::<PyInstrument>()?;

    m.add_class::<PyBiasOrder>()?;
    m.add_class::<PyControlMode>()?;
    m.add_class::<PyDataMode>()?;
    m.add_class::<PyReadType>()?;
    m.add_class::<PyReadAt>()?;
    m.add_class::<PyReadAfter>()?;
    m.add_class::<PyWaitFor>()?;
    m.add_class::<PyAuxDACFn>()?;
    m.add_class::<PyIODir>()?;
    m.add_class::<PyLogicLevel>()?;
    m.add_class::<PyOutputRange>()?;
    m.add("ArC2Error", py.get_type_bound::<ArC2Error>())?;

    m.setattr(intern!(m.py(), "LIBARC2_VERSION"), libarc2::LIBARC2_VERSION)?;

    Ok(())
}

