package app;
/** The controller role wiring a Strategy and a Factory. */
public interface Controller { void run(); }
/** Default controller. */
public class ControllerImpl implements Controller { void run() {} }
/** A sorting Strategy. */
public class SortStrategy {}
/** A widget Factory. */
public class WidgetFactory {}
