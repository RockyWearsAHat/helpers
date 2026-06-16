package app;
/** The view role. */
public interface View { void render(); }
/** A console view. */
public class ConsoleView implements View { void render() {} }
