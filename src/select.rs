//! La sélection multiple à la souris, comme dans l'explorateur de fichiers : clic, Ctrl + clic
//! (ajouter ou retirer), Maj + clic (étendre depuis le dernier élément cliqué).
//! Sert à la Bibliothèque (projets, par identifiant) et à l'assistant « Dossiers sources »
//! (dossiers, par chemin).

/// Les éléments sélectionnés, dans l'ordre où ils l'ont été, et le point de départ de Maj + clic.
#[derive(Clone, Debug)]
pub struct MultiSelect<K> {
    items: Vec<K>,
    anchor: Option<K>,
}

impl<K> Default for MultiSelect<K> {
    fn default() -> Self {
        Self { items: Vec::new(), anchor: None }
    }
}

impl<K: Clone + PartialEq> MultiSelect<K> {
    pub fn contains(&self, k: &K) -> bool {
        self.items.contains(k)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[K] {
        &self.items
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.anchor = None;
    }

    /// Tout ce qui est affiché.
    pub fn set_all(&mut self, order: &[K]) {
        self.items = order.to_vec();
        self.anchor = order.first().cloned();
    }

    /// Un clic sur `k`. `order` : les éléments affichés, dans l'ordre (pour Maj + clic).
    /// Sans modificateur, `k` seul est sélectionné.
    pub fn click(&mut self, k: K, mods: egui::Modifiers, order: &[K]) {
        let range = |a: &K| {
            let (i, j) = (order.iter().position(|x| x == a)?, order.iter().position(|x| *x == k)?);
            Some(order[i.min(j)..=i.max(j)].to_vec())
        };
        if mods.shift
            && let Some(span) = self.anchor.as_ref().and_then(range)
        {
            // Maj + clic remplace la sélection par l'étendue ; Ctrl + Maj + clic l'y ajoute.
            if !mods.command {
                self.items.clear();
            }
            for x in span {
                if !self.items.contains(&x) {
                    self.items.push(x);
                }
            }
            return;
        }
        if mods.command {
            match self.items.iter().position(|x| *x == k) {
                Some(i) => {
                    self.items.remove(i);
                }
                None => self.items.push(k.clone()),
            }
        } else {
            self.items = vec![k.clone()];
        }
        self.anchor = Some(k);
    }

    /// Ne garde que les éléments encore présents (après un filtre, une suppression…).
    pub fn retain(&mut self, keep: impl Fn(&K) -> bool) {
        self.items.retain(|k| keep(k));
        if self.anchor.as_ref().is_some_and(|a| !keep(a)) {
            self.anchor = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Modifiers;

    #[test]
    fn clic_ctrl_et_maj() {
        let order = [1, 2, 3, 4, 5];
        let mut s = MultiSelect::default();
        s.click(2, Modifiers::NONE, &order);
        assert_eq!(s.items(), &[2]);
        s.click(4, Modifiers::COMMAND, &order);
        assert_eq!(s.items(), &[2, 4], "Ctrl + clic ajoute");
        s.click(2, Modifiers::COMMAND, &order);
        assert_eq!(s.items(), &[4], "Ctrl + clic retire");
        s.click(1, Modifiers::SHIFT, &order);
        assert_eq!(s.items(), &[1, 2], "Maj + clic étend depuis le dernier clic (2, même retiré) jusqu'à 1");
        s.click(5, Modifiers::NONE, &order);
        assert_eq!(s.items(), &[5], "un clic simple repart de zéro");
        s.click(4, Modifiers::SHIFT | Modifiers::COMMAND, &order);
        assert_eq!(s.items(), &[5, 4], "Ctrl + Maj + clic ajoute l'étendue");
        s.retain(|k| *k != 5);
        assert_eq!(s.items(), &[4]);
        s.set_all(&order);
        assert_eq!(s.len(), 5);
    }
}
