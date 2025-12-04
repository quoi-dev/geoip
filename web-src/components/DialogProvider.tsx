import React, { createContext, type PropsWithChildren, useContext, useMemo, useRef, useState } from "react";

export interface ManagedDialogProps<T> {
	shown?: boolean;
	onClose?: (result?: T) => void;
}

interface DialogItem<T, P extends ManagedDialogProps<T>> {
	id: number;
	component: React.ComponentType<P>;
	props: P;
	callback?: (result: T) => void;
	closed: boolean;
}

export type ShowDialogFunction = <P extends ManagedDialogProps<any>>(
	component: React.ComponentType<P>,
	props?: Omit<Omit<P, "shown">, "onClose">
) => Promise<P extends ManagedDialogProps<infer T> ? T | undefined : never>;

interface DialogContextData {
	showDialog: ShowDialogFunction;
}

const DialogContext = createContext<DialogContextData>({
	showDialog: () => {
		throw new Error("No DialogProvider present");
	}
});

export const DialogProvider: React.FC<PropsWithChildren> = ({children}) => {
	const nextModalId = useRef(0);
	const [items, setItems] = useState<DialogItem<any, any>[]>([]);
	
	const ctx = useMemo<DialogContextData>(() => ({
		showDialog: (component, props) => new Promise(
			resolve => {
				const id = nextModalId.current++;
				let resolved = false;
				const item: DialogItem<any, any> = {
					id,
					component,
					props,
					callback: result => {
						if (resolved) return;
						resolved = true;
						setItems(items => items.map(it => it.id !== id ? it : {
							...it,
							closed: true
						}));
						resolve(result);
						setTimeout(
							() => setItems(items => items.filter(item => item.id !== id)),
							400
						);
					},
					closed: false
				};
				setItems(items => [...items, item]);
			}
		)
	}), []);
	
	return (
		<DialogContext.Provider value={ctx}>
			{children}
			{
				items.map(({id, component: Component, props, callback, closed}) => (
					<Component
						key={id}
						{...props}
						shown={!closed}
						onClose={callback}
					/>
				))
			}
		</DialogContext.Provider>
	);
};

export const useShowDialog = () => useContext(DialogContext).showDialog;
